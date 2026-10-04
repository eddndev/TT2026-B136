local max_integer = 9007199254740991

local function integer(value)
    if type(value) ~= 'string' or #value > 16 or
       not string.match(value, '^%d+$') or
       (#value > 1 and string.sub(value, 1, 1) == '0') then return nil end
    local parsed = tonumber(value)
    if parsed > max_integer then return nil end
    return parsed
end

local function decimal(value)
    return string.format('%.0f', value)
end

local function idle_deadline(last_activity, idle_ttl, absolute)
    if idle_ttl == 0 then return 0 end
    if last_activity > absolute - idle_ttl then return absolute end
    return last_activity + idle_ttl
end

local function apply(writes)
    -- A script command error cannot undo earlier writes. Check the exact
    -- commands, keys and arguments, including expiration, before the first HSET.
    for _, command in ipairs(writes) do
        if not redis.acl_check_cmd(unpack(command)) then
            return redis.error_reply('session write denied')
        end
    end
    for _, command in ipairs(writes) do redis.call(unpack(command)) end
    return true
end

local operation = ARGV[1]
local absolute_ttl = integer(ARGV[2])
local idle_ttl = integer(ARGV[3])
if not absolute_ttl or absolute_ttl < 1000 or absolute_ttl > 86400000 or
   absolute_ttl % 1000 ~= 0 or not idle_ttl or idle_ttl > absolute_ttl or
   idle_ttl % 1000 ~= 0 then
    return redis.error_reply('invalid session policy')
end
local creating = operation == 'create' or operation == 'create_certificate'
local certificate = operation == 'create_certificate' or operation == 'activity_certificate'
if not creating and operation ~= 'read' and operation ~= 'activity' and not certificate then
    return redis.error_reply('invalid session operation')
end

-- Check command support before creating state on an older server.
local expiration = redis.call('PEXPIRETIME', KEYS[1])
local time = redis.call('TIME')
local seconds = integer(time[1])
local micros = integer(time[2])
if not seconds or not micros or micros > 999999 or
   seconds > math.floor(max_integer / 1000) then return nil end
local now = seconds * 1000 + math.floor(micros / 1000)
if now > max_integer - absolute_ttl then return nil end

local first, last
if certificate then
    first, last = integer(ARGV[7]), integer(ARGV[8])
    if not first or not last or now < first or now >= last or ARGV[5] == '' then return nil end
end

if creating then
    if redis.call('EXISTS', KEYS[1]) ~= 0 then return nil end
    local absolute = now + absolute_ttl
    local ceiling = 0
    local authentication = ''
    if certificate then
        ceiling = integer(ARGV[6])
        if not ceiling or ceiling <= now or ceiling > last then return nil end
        absolute = math.min(absolute, ceiling)
        authentication = ARGV[5]
    end
    local idle = idle_deadline(now, idle_ttl, absolute)
    local write = {'HSET', KEYS[1],
        'version', certificate and '2' or '1', 'identity_json', ARGV[4],
        'absolute_ttl_ms', ARGV[2], 'idle_ttl_ms', ARGV[3],
        'created_at_unix_ms', decimal(now), 'last_activity_unix_ms', decimal(now),
        'absolute_expires_at_unix_ms', decimal(absolute),
        'idle_expires_at_unix_ms', decimal(idle)}
    if certificate then
        write[#write + 1] = 'authentication_json'
        write[#write + 1] = authentication
        write[#write + 1] = 'ceiling_unix_ms'
        write[#write + 1] = decimal(ceiling)
    end
    local result = apply({write, {'PEXPIREAT', KEYS[1], decimal(idle == 0 and absolute or idle)}})
    if result ~= true then return result end
    return {ARGV[4], authentication, decimal(now), decimal(absolute), decimal(idle), decimal(ceiling)}
end

if expiration <= now or redis.call('TYPE', KEYS[1]).ok ~= 'hash' then return nil end
local fields = redis.call('HMGET', KEYS[1],
    'version', 'identity_json', 'absolute_ttl_ms', 'idle_ttl_ms',
    'created_at_unix_ms', 'last_activity_unix_ms',
    'absolute_expires_at_unix_ms', 'idle_expires_at_unix_ms',
    'authentication_json', 'ceiling_unix_ms')
local origin = fields[1] == '2'
if (fields[1] ~= '1' and not origin) or not fields[2] or fields[2] == '' or
   fields[3] ~= ARGV[2] or fields[4] ~= ARGV[3] or
   redis.call('HLEN', KEYS[1]) ~= (origin and 10 or 8) then return nil end
if not origin and (fields[9] or fields[10]) then return nil end
local ceiling = 0
local authentication = ''
if origin then
    ceiling = integer(fields[10])
    authentication = fields[9]
    if not ceiling or not authentication or authentication == '' or #authentication > 16384 then return nil end
end

local created = integer(fields[5])
local last_activity = integer(fields[6])
local absolute = integer(fields[7])
local idle = integer(fields[8])
if not created or not last_activity or not absolute or not idle or
   created > max_integer - absolute_ttl or last_activity < created or
   now < last_activity or now >= absolute then return nil end
local expected_absolute = created + absolute_ttl
if origin then expected_absolute = math.min(expected_absolute, ceiling) end
if absolute ~= expected_absolute or
   idle ~= idle_deadline(last_activity, idle_ttl, absolute) or
   (idle_ttl == 0 and last_activity ~= created) then return nil end
local effective = idle == 0 and absolute or idle
if now >= effective or expiration ~= effective or redis.call('PTTL', KEYS[1]) <= 0 then return nil end

-- JSON stays opaque so authentication generations never become Lua doubles.
if operation == 'activity' or operation == 'activity_certificate' then
    if origin ~= certificate or fields[2] ~= ARGV[4] then return nil end
    if origin and (authentication ~= ARGV[5] or ceiling > last) then return nil end
    if idle_ttl > 0 then
        idle = idle_deadline(now, idle_ttl, absolute)
        local result = apply({{'HSET', KEYS[1], 'last_activity_unix_ms', decimal(now),
            'idle_expires_at_unix_ms', decimal(idle)}, {'PEXPIREAT', KEYS[1], decimal(idle)}})
        if result ~= true then return result end
    end
end
return {fields[2], authentication, decimal(now), decimal(absolute), decimal(idle), decimal(ceiling)}

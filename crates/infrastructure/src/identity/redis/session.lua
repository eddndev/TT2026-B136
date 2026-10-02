local max_integer = 9007199254740991

local function integer(value)
    if type(value) ~= 'string' or #value > 16 or
       not string.match(value, '^%d+$') or
       (#value > 1 and string.sub(value, 1, 1) == '0') then
        return nil
    end
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

local operation = ARGV[1]
local absolute_ttl = integer(ARGV[2])
local idle_ttl = integer(ARGV[3])
if not absolute_ttl or absolute_ttl < 1000 or absolute_ttl > 86400000 or
   absolute_ttl % 1000 ~= 0 or not idle_ttl or idle_ttl > absolute_ttl or
   idle_ttl % 1000 ~= 0 then
    return redis.error_reply('invalid session policy')
end
if operation ~= 'create' and operation ~= 'read' and operation ~= 'activity' then
    return redis.error_reply('invalid session operation')
end

-- Check command support before creating any state on an older server.
local expiration = redis.call('PEXPIRETIME', KEYS[1])
local time = redis.call('TIME')
local now = tonumber(time[1]) * 1000 + math.floor(tonumber(time[2]) / 1000)
if now < 0 or now > max_integer - absolute_ttl then return nil end

if operation == 'create' then
    if redis.call('EXISTS', KEYS[1]) ~= 0 then return nil end
    local absolute = now + absolute_ttl
    local idle = idle_deadline(now, idle_ttl, absolute)
    redis.call('HSET', KEYS[1],
        'version', '1', 'identity_json', ARGV[4],
        'absolute_ttl_ms', ARGV[2], 'idle_ttl_ms', ARGV[3],
        'created_at_unix_ms', decimal(now), 'last_activity_unix_ms', decimal(now),
        'absolute_expires_at_unix_ms', decimal(absolute),
        'idle_expires_at_unix_ms', decimal(idle))
    redis.call('PEXPIREAT', KEYS[1], decimal(idle == 0 and absolute or idle))
    return {ARGV[4], decimal(now), decimal(absolute), decimal(idle)}
end

if expiration <= now or redis.call('TYPE', KEYS[1]).ok ~= 'hash' or
   redis.call('HLEN', KEYS[1]) ~= 8 then
    return nil
end
local fields = redis.call('HMGET', KEYS[1],
    'version', 'identity_json', 'absolute_ttl_ms', 'idle_ttl_ms',
    'created_at_unix_ms', 'last_activity_unix_ms',
    'absolute_expires_at_unix_ms', 'idle_expires_at_unix_ms')
if fields[1] ~= '1' or not fields[2] or fields[2] == '' or
   fields[3] ~= ARGV[2] or fields[4] ~= ARGV[3] then
    return nil
end

local created = integer(fields[5])
local last_activity = integer(fields[6])
local absolute = integer(fields[7])
local idle = integer(fields[8])
if not created or not last_activity or not absolute or not idle or
   created > max_integer - absolute_ttl or absolute ~= created + absolute_ttl or
   last_activity < created or now < last_activity or now >= absolute then
    return nil
end
if idle ~= idle_deadline(last_activity, idle_ttl, absolute) or
   (idle_ttl == 0 and last_activity ~= created) then
    return nil
end
local effective = idle == 0 and absolute or idle
if now >= effective or expiration ~= effective or redis.call('PTTL', KEYS[1]) <= 0 then
    return nil
end

-- JSON stays opaque so authentication generations never become Lua doubles.
if operation == 'activity' then
    if fields[2] ~= ARGV[4] then return nil end
    if idle_ttl > 0 then
        idle = idle_deadline(now, idle_ttl, absolute)
        redis.call('HSET', KEYS[1], 'last_activity_unix_ms', decimal(now),
            'idle_expires_at_unix_ms', decimal(idle))
        redis.call('PEXPIREAT', KEYS[1], decimal(idle))
    end
end
return {fields[2], decimal(now), decimal(absolute), decimal(idle)}

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

local function invalid()
    return redis.error_reply('invalid password reset budget state')
end

if #KEYS ~= 2 or #ARGV ~= 4 or KEYS[1] == KEYS[2] then return invalid() end
local limits = {integer(ARGV[1]), integer(ARGV[3])}
local windows = {integer(ARGV[2]), integer(ARGV[4])}
for i = 1, 2 do
    if not limits[i] or limits[i] < 1 or limits[i] > 4294967295 or
       not windows[i] or windows[i] < 1000 or windows[i] > 86400000 or
       windows[i] % 1000 ~= 0 then
        return invalid()
    end
end

local time = redis.call('TIME')
local seconds = integer(time[1])
local micros = integer(time[2])
if not seconds or not micros or micros > 999999 or
   seconds > math.floor(max_integer / 1000) then
    return invalid()
end
local now = seconds * 1000 + math.floor(micros / 1000)
if now > max_integer - math.max(windows[1], windows[2]) then return invalid() end

local function read_budget(key, maximum, window)
    -- Check exact expiry support even for absent keys, before any write.
    local expiration = redis.call('PEXPIRETIME', key)
    local kind = redis.call('TYPE', key).ok
    if kind == 'none' then
        if expiration ~= -2 then return nil end
        return {count = 0, fresh = true}
    end
    if kind ~= 'hash' or redis.call('HLEN', key) ~= 6 then return nil end
    local fields = redis.call('HMGET', key, 'version', 'limit', 'window_ms',
        'count', 'opened_at_unix_ms', 'expires_at_unix_ms')
    if fields[1] ~= '1' or fields[2] ~= decimal(maximum) or
       fields[3] ~= decimal(window) then
        return nil
    end
    local count = integer(fields[4])
    local opened = integer(fields[5])
    local deadline = integer(fields[6])
    if not count or count < 1 or count > maximum or not opened or not deadline or
       opened > max_integer - window or deadline ~= opened + window or
       opened > now or deadline <= now or expiration ~= deadline or
       redis.call('PTTL', key) <= 0 then
        return nil
    end
    return {count = count, fresh = false}
end

-- Do not let a full budget hide corruption in the other budget.
local budgets = {}
for i = 1, 2 do
    budgets[i] = read_budget(KEYS[i], limits[i], windows[i])
    if not budgets[i] then return invalid() end
end
if budgets[1].count == limits[1] or budgets[2].count == limits[2] then return 0 end

local writes = {}
for i = 1, 2 do
    if budgets[i].fresh then
        local deadline = decimal(now + windows[i])
        writes[#writes + 1] = {'HSET', KEYS[i], 'version', '1',
            'limit', decimal(limits[i]), 'window_ms', decimal(windows[i]),
            'count', '1', 'opened_at_unix_ms', decimal(now),
            'expires_at_unix_ms', deadline}
        writes[#writes + 1] = {'PEXPIREAT', KEYS[i], deadline}
    else
        writes[#writes + 1] = {'HSET', KEYS[i], 'count', decimal(budgets[i].count + 1)}
    end
end

-- Lua command errors do not roll back earlier writes. Check every permission
-- first, including the second key and expiry commands for newly created keys.
for _, command in ipairs(writes) do
    if not redis.acl_check_cmd(unpack(command)) then return invalid() end
end
for _, command in ipairs(writes) do
    redis.call(unpack(command))
end
return 1

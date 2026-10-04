local max_integer = 9007199254740991

local function integer(value)
    if type(value) ~= 'string' or #value > 16 or
       not string.match(value, '^%d+$') or
       (#value > 1 and string.sub(value, 1, 1) == '0') then return nil end
    local parsed = tonumber(value)
    if parsed > max_integer then return nil end
    return parsed
end

local time = redis.call('TIME')
local seconds = integer(time[1])
local micros = integer(time[2])
if not seconds or not micros or micros > 999999 or
   seconds > math.floor(max_integer / 1000) then return nil end
local now = seconds * 1000 + math.floor(micros / 1000)
if now > max_integer then return nil end

if ARGV[1] == 'create' then
    local first = integer(ARGV[3])
    local last = integer(ARGV[4])
    local deadline = integer(ARGV[5])
    if not first or not last or not deadline or now < first or now >= last or
       deadline <= now or deadline > last or deadline - now > 300000 then return 0 end
    -- One SET owns both the value and its absolute expiry; NX never overwrites.
    local stored = redis.call('SET', KEYS[1], ARGV[2], 'NX', 'PXAT', ARGV[5])
    return stored and 1 or 0
end
if ARGV[1] ~= 'take' then return redis.error_reply('invalid MFA operation') end

local expiration = redis.call('PEXPIRETIME', KEYS[1])
local raw = false
if redis.call('TYPE', KEYS[1]).ok == 'string' and
   redis.call('STRLEN', KEYS[1]) <= 16384 then
    raw = redis.call('GET', KEYS[1])
end
-- No malformed or expired record remains available for a second MFA attempt.
redis.call('DEL', KEYS[1])
if not raw or expiration <= now then return nil end
return {raw, string.format('%.0f', now), string.format('%.0f', expiration)}

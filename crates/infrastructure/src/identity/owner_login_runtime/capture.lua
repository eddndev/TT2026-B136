local max_integer = 9007199254740991
local max_json = 4194304

local function integer(value)
    if type(value) ~= 'string' or #value > 16 or
       not string.match(value, '^%d+$') or
       (#value > 1 and string.sub(value, 1, 1) == '0') then return nil end
    local parsed = tonumber(value)
    if parsed > max_integer then return nil end
    return parsed
end

if #KEYS ~= 1 then return redis.error_reply('invalid certificate capture command') end
-- Verify absolute-expiration support before writing even an absent key.
local expiration = redis.call('PEXPIRETIME', KEYS[1])
local time = redis.call('TIME')
local seconds, micros = integer(time[1]), integer(time[2])
if not seconds or not micros or micros > 999999 or
   seconds > math.floor(max_integer / 1000) then return nil end
local now = seconds * 1000 + math.floor(micros / 1000)
if now > max_integer then return nil end

if ARGV[1] == 'create' then
    if #ARGV ~= 4 or #ARGV[2] == 0 or #ARGV[2] > max_json then return 0 end
    local issued, expires = integer(ARGV[3]), integer(ARGV[4])
    if not issued or not expires or expires <= issued or
       expires - issued < 1000 or expires - issued > 300000 or
       issued % 1000 ~= 0 or expires % 1000 ~= 0 or
       now < issued or now >= expires then return 0 end
    -- One command admits both the value and exact expiry, with no overwrite.
    local stored = redis.call('SET', KEYS[1], ARGV[2], 'NX', 'PXAT', ARGV[4])
    return stored and 1 or 0
end
if #ARGV ~= 1 or ARGV[1] ~= 'take' then
    return redis.error_reply('invalid certificate capture command')
end

local raw = false
if redis.call('TYPE', KEYS[1]).ok == 'string' and
   redis.call('STRLEN', KEYS[1]) <= max_json then
    raw = redis.call('GETDEL', KEYS[1])
else
    redis.call('DEL', KEYS[1])
end
-- Consumption precedes all decoding, so malformed material is one-use too.
if not raw or expiration <= now then return nil end
return {raw, string.format('%.0f', now), string.format('%.0f', expiration)}

-- SPDX-License-Identifier: GPL-3.0-or-later

if redis.call("GET", KEYS[2]) then
    if redis.call("GET", KEYS[1]) then
        redis.call("EXPIRE", KEYS[2], "172800")
        return 1
    else
        redis.call("EXPIRE", KEYS[2], "604800")
        return 2
    end
else
    redis.call("SET", KEYS[1], "1", "EX", "300")
    redis.call("SET", KEYS[2], "1", "EX", "172800")
    return 0
end

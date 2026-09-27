"""Give each active nextest slot independent disposable backend databases."""

import os
import sys
from urllib.parse import urlsplit, urlunsplit


def database_url(base, name):
    parts = urlsplit(base)
    if parts.scheme not in {"postgres", "postgresql", "redis"} or not parts.netloc:
        raise ValueError("a complete backend URL is required")
    return urlunsplit(parts._replace(path=f"/{name}"))


def slot_environment(environment):
    try:
        count = int(environment["TT_CI_TEST_SLOTS"])
        slot = int(environment["NEXTEST_TEST_GLOBAL_SLOT"])
        postgres = environment["TT_CI_POSTGRES_BASE"]
        redis = environment["TT_CI_REDIS_BASE"]
    except (KeyError, ValueError) as error:
        raise ValueError("nextest requires a slot and all disposable backend addresses") from error
    if not 1 <= count <= 12 or not 0 <= slot < count:
        raise ValueError("nextest slot must fit the configured 1..12 backend slots")
    result = dict(environment)
    for kind in ("IDENTITY", "CASE", "DOCUMENT"):
        result[f"{kind}_TEST_DATABASE_URL"] = database_url(postgres, f"ci_{kind.lower()}_{slot}")
    result["IDENTITY_TEST_REDIS_URL"] = database_url(redis, slot)
    return result


def main():
    if len(sys.argv) < 2:
        raise SystemExit("expected a test executable")
    # Nextest also invokes target runners for discovery, before assigning slots.
    environment = os.environ if "--list" in sys.argv[2:] else slot_environment(os.environ)
    os.execvpe(sys.argv[1], sys.argv[1:], environment)


if __name__ == "__main__":
    main()

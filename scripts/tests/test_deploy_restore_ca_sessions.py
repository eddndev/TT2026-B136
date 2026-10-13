"""Restoration removes CA capabilities and preserves independent MFA controls."""
import copy
import unittest

from deploy_restore_redis_support import RedisInvalidationFixture


class RestoreCaSessionsTests(RedisInvalidationFixture):
    def test_ca_and_office_capabilities_are_invalidated_without_resetting_limits(self):
        ca = {
            'ca-identity:session:' + 'a' * 64: ('ca-session', 2200000000050),
            'ca-identity:challenge:' + 'b' * 64: ('ca-mfa', 2200000000051),
            'ca-identity:password-failures:' + 'c' * 64: ('4', 2200000000052),
            'ca-identity:totp-used:' + 'd' * 64: ('1', 2200000000053),
            'foreign-ca-identity:session:' + 'e' * 64: ('other', 2200000000054),
        }
        self.redis.values.update(ca)
        before = copy.deepcopy(self.redis.values)
        result = self.invoke()
        self.assertEqual(result, {'sessions_removed': 3, 'challenges_removed': 2,
                                  'certificate_logins_removed': 0})
        removed_prefixes = ('identity:session:', 'identity:challenge:',
                            'ca-identity:session:', 'ca-identity:challenge:')
        expected = {key: value for key, value in before.items()
                    if not key.startswith(removed_prefixes)}
        self.assertEqual(self.redis.values, expected)
        first_delete = next(i for i, call in enumerate(self.redis.calls) if call[0] == 'DEL')
        for pattern in ('ca-identity:session:*', 'ca-identity:challenge:*'):
            self.assertTrue(any(call[0] == 'SCAN' and pattern in call
                                for call in self.redis.calls[:first_delete]))
        self.assertEqual(self.invoke(), {'sessions_removed': 0, 'challenges_removed': 0,
                                        'certificate_logins_removed': 0})
        self.assertEqual(self.redis.values, expected)

    def test_malformed_ca_capability_aborts_before_any_office_key_is_deleted(self):
        self.redis.values['ca-identity:session:bad'] = ('unadmitted', 2200000000050)
        before = copy.deepcopy(self.redis.values)
        self.rejection()
        self.assertEqual(self.redis.delete_calls, 0)
        self.assertEqual(self.redis.values, before)


if __name__ == '__main__':
    unittest.main()

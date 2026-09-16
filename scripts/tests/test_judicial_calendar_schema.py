"""Exercise immutable global calendar rows on disposable PostgreSQL."""
from copy import deepcopy
from hashlib import sha256
import json
import unittest
from uuid import UUID, uuid4
from judicial_calendar_sql_support import CalendarSqlFixture, HELPERS, MIGRATIONS, ORACLE, ROOT, blob, literal, raw_values


class CalendarSchemaTests(CalendarSqlFixture):
    def test_roots_require_revision_one_at_commit(self):
        calendar = str(uuid4())
        self.reject('INSERT INTO judicial_calendars(id) VALUES (' + literal(calendar)
                    + '); SET CONSTRAINTS ALL IMMEDIATE', 'foreign_key_violation')
        self.reject('INSERT INTO judicial_calendars(id,initial_revision) VALUES ('
                    + literal(calendar) + ',2)')
        row = self.sql("SELECT condeferrable AND condeferred FROM pg_constraint WHERE "
                       "conrelid='judicial_calendars'::regclass AND conname='judicial_calendar_first_revision'")
        self.assertEqual(row, 't')

    def test_publish_replace_retire_preserve_exact_history_and_zero_uuid(self):
        calendar, actor = self.publish(calendar=str(UUID(int=0)))
        value = ORACLE.minimum()
        value['weekly_pattern'][0]['explanation'] = 'Changed declaration'
        self.sql(self.insert(calendar, actor, revision=2, action='replace', value=value, reason='Correction'))
        self.sql(self.insert(calendar, actor, revision=3, action='retire', value=value, reason='Retired internally'))
        rows = self.sql("SELECT jsonb_build_object('revision',revision,'status',status,'values',values_view) "
                        'FROM judicial_calendar_revisions WHERE calendar_id=' + literal(calendar) + ' ORDER BY revision')
        rows = [json.loads(line) for line in rows.splitlines()]
        self.assertEqual([r['status'] for r in rows], ['published', 'published', 'retired'])
        self.assertEqual(rows[0]['values'], ORACLE.minimum())
        self.assertEqual(rows[1]['values'], value)
        self.assertEqual(rows[2]['values'], value)
        self.reject(self.insert(calendar, actor, revision=4, action='replace', value=value, reason='Again'))
        self.reject(self.insert(calendar, actor, revision=4, action='retire', value=value, reason='Again'))

    def test_sequence_scope_and_retirement_content_are_enforced(self):
        calendar, actor = self.publish()
        changed = ORACLE.minimum()
        changed['scope']['title'] = 'Different scope'
        self.reject(self.insert(calendar, actor, revision=2, action='replace', value=changed, reason='Changed'))
        self.reject(self.insert(calendar, actor, revision=3, action='replace', reason='Skipped'))
        changed = ORACLE.minimum()
        changed['weekly_pattern'][0]['explanation'] = 'Different content'
        self.reject(self.insert(calendar, actor, revision=2, action='retire', value=changed, reason='Retired'))
        self.reject(self.insert(calendar, actor, revision=2, action='replace', reason='Changed', overrides={'action': "'publish'"}))
        self.assertEqual(self.sql('SELECT max(revision) FROM judicial_calendar_revisions WHERE calendar_id=' + literal(calendar)), '1')

    def test_receipt_projection_digests_metadata_and_generated_values_are_enforced(self):
        calendar, actor = self.publish()
        for override in [dict(values_digest="decode(repeat('00',32),'hex')"),
                         dict(submission_digest="decode(repeat('00',32),'hex')"),
                         dict(operation_id=literal(str(uuid4()))), dict(recorded_by_email="'wrong@example.invalid'"),
                         dict(reason="'Other reason'"), dict(revision='3'),
                         dict(recorded_at_seconds='253402300800'), dict(recorded_at_nanoseconds='1000000000')]:
            self.reject(self.insert(calendar, actor, revision=2, action='replace', reason='Correction', overrides=override),
                        'check_violation OR insufficient_privilege')
        self.reject(self.insert(calendar, actor, revision=2, action='replace', reason='Correction',
                               overrides=dict(values_view="'{}'::jsonb")), 'generated_always')
        self.reject(self.insert(calendar, actor, revision=2, action='replace', reason='Correction',
                               overrides=dict(values_canonical='NULL')), 'not_null_violation OR check_violation')

    def test_only_current_active_owner_can_append(self):
        calendar, owner = self.publish()
        for role, active in [('litigator', True), ('paralegal', True), ('client', True), ('owner', False)]:
            actor = self.actor(role, active)
            self.reject(self.insert(calendar, actor, revision=2, action='replace', reason='Attempt'), 'insufficient_privilege')
        self.sql('UPDATE users SET active=FALSE WHERE id=' + literal(owner[0]))
        self.reject(self.insert(calendar, owner, revision=2, action='replace', reason='Attempt'), 'insufficient_privilege')
        self.sql('UPDATE users SET active=TRUE,role=\'litigator\' WHERE id=' + literal(owner[0]))
        self.reject(self.insert(calendar, owner, revision=2, action='replace', reason='Attempt'), 'insufficient_privilege')

    def test_operation_is_unique_across_roots(self):
        operation = str(uuid4())
        _, actor = self.publish(operation=operation)
        calendar = str(uuid4())
        self.reject('INSERT INTO judicial_calendars(id) VALUES (' + literal(calendar) + ');'
                    + self.insert(calendar, actor, operation=operation), 'unique_violation')
        self.assertEqual(self.sql('SELECT count(*) FROM judicial_calendars WHERE id=' + literal(calendar)), '0')

    def test_no_update_delete_or_truncate_even_for_empty_statement(self):
        calendar, _ = self.publish()
        for table in ['judicial_calendars', 'judicial_calendar_revisions']:
            for command in ['UPDATE ' + table + ' SET ' + ('id=id' if table=='judicial_calendars' else 'revision=revision') + ' WHERE FALSE',
                            'DELETE FROM ' + table + ' WHERE FALSE', 'TRUNCATE ' + table + ' CASCADE']:
                self.reject(command)
        self.assertEqual(self.sql('SELECT count(*) FROM judicial_calendars WHERE id=' + literal(calendar)), '1')

    def test_migration_reapplication_preserves_rows_and_public_has_no_access(self):
        calendar, _ = self.publish()
        before = self.sql('SELECT row_to_json(r) FROM judicial_calendar_revisions r WHERE calendar_id=' + literal(calendar))
        for suffix in MIGRATIONS:
            self.sql((ROOT / 'migrations' / ('0013_judicial_calendar_' + suffix + '.sql')).read_text())
        after = self.sql('SELECT row_to_json(r) FROM judicial_calendar_revisions r WHERE calendar_id=' + literal(calendar))
        self.assertEqual(before, after)
        self.assertEqual(self.sql("SELECT count(*) FROM pg_class c CROSS JOIN LATERAL aclexplode(coalesce(c.relacl,acldefault('r',c.relowner))) a "
            "WHERE c.oid IN ('judicial_calendars'::regclass,'judicial_calendar_revisions'::regclass) AND a.grantee=0"), '0')
        self.assertEqual(self.sql("SELECT count(*) FROM pg_proc p CROSS JOIN LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a "
            "WHERE p.pronamespace=current_schema()::regnamespace AND (p.proname LIKE 'judicial_calendar_%' OR p.proname LIKE '%judicial_calendar_history' "
            "OR p.proname='enforce_judicial_calendar_sequence') AND a.grantee=0"), '0')

    def test_serializable_and_repeatable_read_writes_are_rejected(self):
        calendar, actor = self.publish()
        for isolation in ['REPEATABLE READ', 'SERIALIZABLE']:
            command = self.insert(calendar, actor, revision=2, action='replace', reason='Attempt')
            self.sql('BEGIN ISOLATION LEVEL ' + isolation + '; DO $test$ BEGIN ' + command
                     + "; RAISE EXCEPTION 'unexpected acceptance'; EXCEPTION WHEN check_violation THEN NULL; END; $test$;ROLLBACK;")

    def test_restricted_role_reads_and_appends_without_owning_or_mutating_history(self):
        role = 'calendar_runtime_' + uuid4().hex
        calendar, actor = self.publish()
        self.sql('CREATE ROLE ' + role + ';GRANT USAGE ON SCHEMA ' + self.schema + ' TO ' + role + ';'
                 'GRANT SELECT,INSERT ON judicial_calendars,judicial_calendar_revisions TO ' + role + ';'
                 'GRANT SELECT,UPDATE(recovery_codes) ON users TO ' + role + ';'
                 'GRANT EXECUTE ON FUNCTION ' + ','.join(HELPERS + ['hearing_text(bytea,integer,integer,boolean)',
                 'typed_u32(bytea,integer)', 'case_administration_text_valid(text,integer,boolean)']) + ' TO ' + role)
        try:
            self.sql('SET ROLE ' + role + ';' + self.insert(calendar, actor, revision=2, action='replace', reason='Correction'))
            self.assertEqual(self.sql('SET ROLE ' + role + ';SELECT max(revision) FROM judicial_calendar_revisions WHERE calendar_id=' + literal(calendar)), '2')
            for permission in ['UPDATE', 'DELETE', 'TRUNCATE', 'TRIGGER']:
                self.assertEqual(self.sql('SELECT has_table_privilege(' + literal(role) + ", 'judicial_calendar_revisions'," + literal(permission) + ')'), 'f')
            for function in ['preserve_judicial_calendar_history()', 'enforce_judicial_calendar_sequence()']:
                self.assertEqual(self.sql('SELECT has_function_privilege(' + literal(role) + ',' + literal(function) + ",'EXECUTE')"), 'f')
        finally:
            self.sql('DROP OWNED BY ' + role + '; DROP ROLE ' + role)


if __name__ == '__main__':
    unittest.main()

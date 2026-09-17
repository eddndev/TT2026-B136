"""Exercise direct SQL writer races without the application transaction wrapper."""
from concurrent.futures import ThreadPoolExecutor
import subprocess
from threading import Barrier
import time
import unittest
from uuid import uuid4
from judicial_calendar_sql_support import CalendarSqlFixture, literal


class CalendarConcurrencyTests(CalendarSqlFixture):
    def run_statement(self, statement):
        return subprocess.run(['psql', self.url, '-X', '-qAt', '-v', 'ON_ERROR_STOP=1'],
            input='SET search_path TO ' + self.schema + ',pg_catalog;\n' + statement,
            text=True, capture_output=True, timeout=20)

    def simultaneous(self, first, second):
        barrier = Barrier(2)
        def run(statement):
            barrier.wait(timeout=10)
            return self.run_statement('BEGIN;SET LOCAL lock_timeout=\'10s\';' + statement + ';COMMIT;')
        with ThreadPoolExecutor(max_workers=2) as pool:
            one, two = pool.submit(run, first), pool.submit(run, second)
            results = [one.result(), two.result()]
        self.assertEqual(sorted(result.returncode for result in results), [0, 3])
        return next(result.stderr for result in results if result.returncode)

    def test_simultaneous_replace_and_retire_cannot_both_append(self):
        calendar, actor = self.publish()
        loser = self.simultaneous(self.insert(calendar, actor, revision=2, action='replace', reason='Correction'),
                                  self.insert(calendar, actor, revision=2, action='retire', reason='Retirement'))
        self.assertIn('calendar revisions must follow a published predecessor', loser)
        self.assertEqual(self.sql('SELECT count(*) FROM judicial_calendar_revisions WHERE calendar_id=' + literal(calendar)), '2')

    def test_operation_collision_rolls_back_the_losing_root(self):
        actor = self.actor()
        first, second, operation = str(uuid4()), str(uuid4()), str(uuid4())
        def statement(calendar):
            return 'INSERT INTO judicial_calendars(id) VALUES (' + literal(calendar) + ');' + self.insert(calendar, actor, operation=operation)
        loser = self.simultaneous(statement(first), statement(second))
        self.assertIn('judicial_calendar_operation_unique', loser)
        self.assertEqual(self.sql('SELECT count(*) FROM judicial_calendars WHERE id IN (' + literal(first) + ',' + literal(second) + ')'), '1')
        self.assertEqual(self.sql('SELECT count(*) FROM judicial_calendar_revisions WHERE operation_id=' + literal(operation)), '1')

    def test_actor_is_reloaded_after_waiting_for_common_audit_lock(self):
        calendar, actor = self.publish()
        marker = 'calendar_wait_' + uuid4().hex
        locker = subprocess.Popen(['psql', self.url, '-X', '-qAt', '-v', 'ON_ERROR_STOP=1'],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        try:
            locker.stdin.write('SET search_path TO ' + self.schema + ',pg_catalog; BEGIN;'
                               "SELECT pg_advisory_xact_lock(280603412820);SELECT 'LOCKED';\n")
            locker.stdin.flush()
            while locker.stdout.readline().strip() != 'LOCKED':
                if locker.poll() is not None:
                    self.fail('Lock holder stopped before synchronization')
            with ThreadPoolExecutor(max_workers=1) as pool:
                future = pool.submit(self.run_statement, 'SET application_name=' + literal(marker) + ';'
                                     + self.insert(calendar, actor, revision=2, action='replace', reason='Waiting'))
                deadline = time.monotonic() + 10
                while True:
                    waiting = self.sql("SELECT count(*) FROM pg_stat_activity WHERE application_name=" + literal(marker)
                                       + " AND wait_event_type='Lock' AND wait_event='advisory'")
                    if waiting == '1':
                        break
                    if time.monotonic() >= deadline:
                        self.fail('Calendar writer never reached the common audit lock')
                    time.sleep(0.02)
                self.sql('UPDATE users SET active=FALSE WHERE id=' + literal(actor[0]))
                locker.stdin.write('COMMIT;\n\\q\n')
                locker.stdin.flush()
                outcome = future.result(timeout=10)
            self.assertNotEqual(outcome.returncode, 0)
            self.assertIn('judicial calendar actor is not currently authorized', outcome.stderr)
            self.assertEqual(self.sql('SELECT count(*) FROM judicial_calendar_revisions WHERE calendar_id=' + literal(calendar)), '1')
        finally:
            if locker.poll() is None:
                locker.terminate()
            locker.communicate(timeout=10)


if __name__ == '__main__':
    unittest.main()

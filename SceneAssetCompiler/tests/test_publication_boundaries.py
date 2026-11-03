"""Focused real-filesystem regressions; fixtures do not claim human/PSD/provider validation."""
from pathlib import Path
import json
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SKILL = ROOT/'.agents/skills/scene-asset-compiler'
sys.path.insert(0, str(SKILL/'scripts'))
from scene_asset_compilerlib.publish import human_approve, publish
from scene_asset_compilerlib.util import atomic_json, sha256_file
from scene_asset_compilerlib.workspace import begin_attempt, compile_plan

class PublicationBoundaries(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)

    def selected_run(self, human=True):
        r = self.root
        req = {'project_id':'p', 'review_policy':{'human_final_review':human}}
        atomic_json(r/'request.json', req)
        atomic_json(r/'run.json', {'run_id':'r', 'project_id':'p', 'request_sha256':sha256_file(r/'request.json')})
        atomic_json(r/'plan/compiled.json', {'plan_id':'plan', 'task_ids':['t']})
        atomic_json(r/'tasks/t/task.json', {'required':True, 'asset_id':'asset'})
        artifact = r/'tasks/t/attempts/0001/outputs/value.png'
        artifact.parent.mkdir(parents=True); artifact.write_bytes(b'bounded-test-bytes-not-a-provider-image')
        self.select(artifact)
        return artifact

    def select(self, artifact):
        atomic_json(self.root/'tasks/t/selection.json', {'attempt_no':1, 'artifacts':[{
            'artifact_id':'a', 'role':'PRIMARY', 'source_path':'attempts/0001/outputs/value.png',
            'sha256':sha256_file(artifact), 'mime':'image/png', 'size_bytes':artifact.stat().st_size
        }]})

    def test_approval_is_bound_and_publication_is_idempotent(self):
        self.selected_run()
        receipt = human_approve(self.root, 'reviewer', 'exact selection')
        self.assertEqual(set(receipt), {'schema_version','status','reviewer','approved_at','note'})
        first = publish(self.root)
        self.assertEqual(first, publish(self.root))
        self.assertEqual(first['lineage'], {'run_id':'r', 'plan_id':'plan'})

    def test_old_unbound_approval_cannot_publish(self):
        self.selected_run()
        atomic_json(self.root/'publish/human-approval.json', {'status':'APPROVED','reviewer':'r'})
        with self.assertRaisesRegex(ValueError, 'HUMAN_FINAL_REVIEW'): publish(self.root)
        self.assertFalse((self.root/'publish/asset-pack.json').exists())

    def test_changed_selection_with_matching_new_hash_requires_new_approval(self):
        artifact = self.selected_run(); human_approve(self.root, 'r')
        artifact.write_bytes(b'new correctly rehashed candidate'); self.select(artifact)
        with self.assertRaisesRegex(ValueError, 'HUMAN_APPROVAL_STALE'): publish(self.root)
        human_approve(self.root, 'r', 'reviewed changed candidate')
        self.assertEqual(len(publish(self.root)['assets']), 1)

    def test_changed_receipt_invalidates_internal_commit_marker(self):
        self.selected_run(); human_approve(self.root, 'r')
        path = self.root/'publish/human-approval.json'
        value = json.loads(path.read_text()); value['reviewer'] = 'another'; atomic_json(path, value)
        with self.assertRaisesRegex(ValueError, 'HUMAN_APPROVAL_STALE'): publish(self.root)

    def test_cannot_disable_review_by_mutating_immutable_request(self):
        self.selected_run(); human_approve(self.root, 'r')
        atomic_json(self.root/'request.json', {'project_id':'p','review_policy':{'human_final_review':False}})
        with self.assertRaisesRegex(ValueError, 'RUN_REQUEST_DRIFT'): publish(self.root)

    def test_rejects_blank_reviewer_empty_selection_and_artifact_drift(self):
        artifact = self.selected_run()
        with self.assertRaisesRegex(ValueError, 'REVIEWER'): human_approve(self.root, '  ')
        artifact.write_bytes(b'drift')
        with self.assertRaisesRegex(ValueError, 'HASH'): human_approve(self.root, 'r')
        atomic_json(self.root/'tasks/t/selection.json', {'attempt_no':1,'artifacts':[]})
        with self.assertRaisesRegex(ValueError, 'EMPTY_SELECTION'): human_approve(self.root, 'r')

    def test_no_human_review_policy_still_supports_valid_publication(self):
        self.selected_run(human=False)
        self.assertEqual(len(publish(self.root)['assets']), 1)

    def test_human_approval_does_not_follow_publish_symlink(self):
        self.selected_run()
        outside = self.root/'outside'; outside.mkdir()
        (self.root/'publish').symlink_to(outside, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'SYMLINK'): human_approve(self.root, 'r')
        self.assertEqual(list(outside.iterdir()), [])

    def test_reference_failure_does_not_allocate_an_attempt(self):
        task_dir = self.root/'tasks/t'; (task_dir/'attempts').mkdir(parents=True)
        atomic_json(task_dir/'task.json', {'depends_on':[], 'references':[{'source_kind':'FILE','source':'missing.png'}]})
        with self.assertRaisesRegex(ValueError, 'REFERENCE_MISSING'): begin_attempt(self.root, 't')
        self.assertEqual(list((task_dir/'attempts').iterdir()), [])

    def template_run(self):
        for folder in ['tasks','plan','authority']: (self.root/folder).mkdir()
        shutil.copy2(SKILL/'templates/visual-authority.json', self.root/'authority/visual-authority.json')
        shutil.copy2(SKILL/'templates/generation-plan.json', self.root/'plan/generation-plan.json')

    def test_compile_and_attempt_use_public_templates(self):
        self.template_run()
        ids = compile_plan(self.root)
        ad = begin_attempt(self.root, ids[0])
        self.assertTrue((ad/'attempt.json').is_file())
        self.assertTrue((ad/'prompt.md').is_file())
        self.assertTrue((ad/'generation-capsule.md').is_file())

    def test_compile_failure_removes_only_its_staging_work(self):
        self.template_run()
        def failing_write(*args, **kwargs): raise OSError('simulated disk write failure')
        with patch('scene_asset_compilerlib.workspace.atomic_json', side_effect=failing_write):
            with self.assertRaisesRegex(OSError, 'disk'): compile_plan(self.root)
        self.assertEqual(list((self.root/'tasks').iterdir()), [])
        self.assertFalse((self.root/'plan/compiled.json').exists())
        self.assertFalse(list(self.root.glob('.compile-*')))
        self.assertTrue((self.root/'plan/generation-plan.json').is_file())
        self.assertTrue(compile_plan(self.root))

    def test_compile_does_not_replace_dangling_task_symlink(self):
        self.template_run()
        plan = json.loads((self.root/'plan/generation-plan.json').read_text())
        dest = self.root/'tasks'/plan['tasks'][0]['task_id']
        dest.symlink_to(self.root/'missing', target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'TASK_ALREADY_EXISTS'): compile_plan(self.root)
        self.assertTrue(dest.is_symlink())
        self.assertFalse(list(self.root.glob('.compile-*')))

    def test_compile_rejects_symlinked_task_root(self):
        self.template_run()
        (self.root/'tasks').rmdir()
        outside = self.root/'outside'; outside.mkdir()
        (self.root/'tasks').symlink_to(outside, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'UNSAFE_PLAN_DIRECTORY'): compile_plan(self.root)
        self.assertEqual(list(outside.iterdir()), [])

    def test_attempt_write_failure_does_not_leave_generating_state(self):
        self.template_run(); ids = compile_plan(self.root)
        with patch('scene_asset_compilerlib.workspace.atomic_text', side_effect=OSError('write failed')):
            with self.assertRaisesRegex(OSError, 'write failed'): begin_attempt(self.root, ids[0])
        self.assertEqual(list((self.root/'tasks'/ids[0]/'attempts').iterdir()), [])
        self.assertTrue(begin_attempt(self.root, ids[0]).is_dir())

if __name__ == '__main__': unittest.main()

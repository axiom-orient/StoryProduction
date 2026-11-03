#!/usr/bin/env python3
"""Explicit NVC cinematic authoring CLI; never part of an agent approval toolset."""
from __future__ import annotations
import argparse
import datetime
import json
import sys
from pathlib import Path
from cinematic_contracts import (CinematicError, extract_continuity, load_completed_release,
    publish, read_artifact, require, safe_path, seal, validate_approval, validate_plan)

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=('continuity','plan','approve','inspect'))
    parser.add_argument('--project-root', required=True, type=Path)
    parser.add_argument('--release-bundle', required=True)
    parser.add_argument('--continuity')
    parser.add_argument('--plan')
    parser.add_argument('--draft')
    parser.add_argument('--approval')
    parser.add_argument('--output')
    parser.add_argument('--reviewer')
    parser.add_argument('--expected-plan-hash')
    args = parser.parse_args()
    publication_completed = False
    try:
        release = load_completed_release(args.project_root, args.release_bundle)
        scene_map = release.scene_map
        source = release.binding
        bible = extract_continuity(scene_map, source)
        if args.command == 'continuity':
            result = bible
        else:
            require(bool(args.continuity), '--continuity is required')
            supplied, _ = read_artifact(args.project_root, args.continuity)
            require(supplied == bible, 'CONTINUITY_NOT_CURRENT')
            if args.command == 'plan':
                require(bool(args.draft), '--draft is required: no implicit scene-to-shot fallback exists')
                draft, _ = read_artifact(args.project_root, args.draft)
                require('contentHash' not in draft, 'DRAFT_MUST_NOT_CLAIM_SEALED_IDENTITY')
                result = seal(draft)
                validate_plan(scene_map, bible, result, source)
            else:
                require(bool(args.plan), '--plan is required')
                plan, _ = read_artifact(args.project_root, args.plan)
                if args.command == 'approve':
                    require(bool(args.reviewer and args.reviewer.strip()), '--reviewer is required')
                    require(args.expected_plan_hash == plan['contentHash'], 'REVIEW_HASH_MISMATCH')
                    # Retrying the same explicit review reuses the immutable receipt.
                    if args.output and safe_path(args.project_root, args.output, existing=False).exists():
                        existing, _ = read_artifact(args.project_root, args.output)
                        validate_approval(plan, bible, existing)
                        require(existing['reviewedBy'] == args.reviewer.strip(), 'APPROVAL_REVIEWER_CONFLICT')
                        result = existing
                    else:
                        result = seal({'schemaVersion':1,'decision':'APPROVED',
                            'directorPlanSha256':plan['contentHash'],'continuityBibleSha256':bible['contentHash'],
                            **source,'reviewedBy':args.reviewer.strip(),
                            'reviewedAt':datetime.datetime.now(datetime.timezone.utc).isoformat()})
                        validate_approval(plan, bible, result)
                else:
                    if args.approval:
                        approval, _ = read_artifact(args.project_root, args.approval)
                        validate_approval(plan,bible,approval)
                    else:
                        validate_plan(scene_map, bible, plan, source)
                    result = {'status':'VALID','planHash':plan['contentHash'],
                              'scenes':len(plan['scenes']), 'shots':sum(len(s['shots']) for s in plan['scenes']),
                              'approvalChecked': bool(args.approval)}
        if args.command == 'inspect':
            require(not args.output, 'inspect is read-only; --output is not supported')
            print(json.dumps(result,ensure_ascii=False)); return 0
        require(bool(args.output), '--output is required')
        raw_hash = publish(args.project_root,args.output,result)
        publication_completed = True
        print(json.dumps({'path':args.output,'sha256':raw_hash,'contentHash':result['contentHash']},ensure_ascii=False))
        return 0
    except (CinematicError, OSError, KeyError) as error:
        unknown = publication_completed or getattr(error, 'outcome', None) == 'outcome_unknown'
        print(json.dumps({'status':'outcome_unknown' if unknown else 'REJECTED','error':str(error)},ensure_ascii=False),file=sys.stderr)
        return 3 if unknown else 2

if __name__ == '__main__':
    raise SystemExit(main())

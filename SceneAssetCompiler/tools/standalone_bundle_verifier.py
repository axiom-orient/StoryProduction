from __future__ import annotations

from pathlib import Path
import io, json, os, re, subprocess, sys, tarfile, tempfile, zipfile

from release_verifier_support import GateError, sha, sha_bytes

PYTHON = sys.executable

STANDALONE_PACKAGE_NAME='SceneAssetCompiler-Standalone-0.13.0'
STANDALONE_INSTALL_LINES=(
    'BUNDLE_ROOT="$(pwd -P)"',
    'PRODUCT_ROOT="$BUNDLE_ROOT/SceneAssetCompiler"',
    'SCENE_ASSET_COMPILER="$PRODUCT_ROOT/.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py"',
    'PROVIDER_BIN="$BUNDLE_ROOT/bin/scene-provider-v2"',
    'PERFECTPIXEL_BIN="$BUNDLE_ROOT/bin/perfectpixel"',
    'COMPONENT_LOCK="$PRODUCT_ROOT/contracts/runtime-component-lock.json"',
    'probe-scene-provider',
    'execute-scene-provider',
    '--provider-bin "$PROVIDER_BIN"',
    '--component-lock "$COMPONENT_LOCK"',
)
STANDALONE_DOCS=(
    '.agents/skills/scene-asset-compiler/SKILL.md',
    '.agents/skills/scene-asset-compiler/references/34-scene-provider-executor.md',
    'docs/OPERATIONS.md',
)

def run_bounded(cmd, *, cwd, timeout=30, extra_env=None):
    env=os.environ.copy(); env['PYTHONDONTWRITEBYTECODE']='1'
    if extra_env:
        env.update({str(key):str(value) for key,value in extra_env.items()})
    try:
        p=subprocess.run(cmd,cwd=cwd,text=True,capture_output=True,env=env,timeout=timeout)
    except subprocess.TimeoutExpired as e:
        raise GateError(f'STANDALONE_TIMEOUT:{" ".join(map(str,cmd))}') from e
    if p.returncode:
        raise GateError(f'STANDALONE_COMMAND_FAIL:{" ".join(map(str,cmd))}\n{p.stdout}\n{p.stderr}')
    return p.stdout+p.stderr


def _extracted_integration_smoke(root, lock):
    """Exercise the extracted binaries through the real SAC execution path."""
    cli=root/'SceneAssetCompiler/.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py'
    with tempfile.TemporaryDirectory(prefix='scene-asset-compiler-extracted-integration-') as temp:
        work=Path(temp).resolve()
        provider=work/'scene-provider-v2'
        provider.write_text("""#!/usr/bin/env python3
import base64, json, os, sys
request=json.load(sys.stdin)
if request.get('operation') == 'protocol_probe':
    print(json.dumps({'schemaVersion':2,'requestId':request['requestId'],'ok':False,'error':{'code':'invalid_request','message':'unsupported operation','retryable':False}}))
    raise SystemExit(0)
if request.get('operation') != 'image_generate':
    print(json.dumps({'schemaVersion':2,'requestId':request['requestId'],'ok':False,'error':{'code':'invalid_request','message':'unsupported operation','retryable':False}}))
    raise SystemExit(0)
data=open(os.environ['FAKE_PNG_PATH'],'rb').read()
count_path=os.environ['FAKE_CALLS_PATH']
with open(count_path,'a',encoding='utf-8') as stream:
    stream.write('image_generate\\n')
print(json.dumps({'schemaVersion':2,'requestId':request['requestId'],'operation':'image_generate','image':{'mediaType':'image/png','base64':base64.b64encode(data).decode('ascii'),'byteLength':len(data),'width':1024,'height':1536,'transparentBackground':False}}))
""",encoding='utf-8')
        provider.chmod(0o755)
        custom_lock=json.loads(json.dumps(lock))
        custom_lock['provider']['sha256']=sha(provider)
        lock_path=work/'runtime-component-lock.json'
        lock_path.write_text(json.dumps(custom_lock,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
        run=work/'run'; attempt=run/'tasks/TASK-extracted/attempts/0001'; (attempt/'outputs').mkdir(parents=True)
        (run/'runtime').mkdir(); (run/'workspace').mkdir()
        (run/'request.json').write_text(json.dumps({'schema_version':1,'review_policy':{'allow_warnings':True}})+'\n',encoding='utf-8')
        (run/'tasks/TASK-extracted/task.json').write_text(json.dumps({'task_id':'TASK-extracted','kind':'KEY_VISUAL','structure':{'required_layer_ids':['scene.whole']},'output':{'preferred':'PNG','fallback_allowed':False,'transparent_background':False,'size':'1024x1536'}})+'\n',encoding='utf-8')
        (attempt/'attempt.json').write_text(json.dumps({'schema_version':2,'task_id':'TASK-extracted','attempt_no':1,'mode':'GENERATE','resolved_references':[],'repair':None})+'\n',encoding='utf-8')
        (attempt/'prompt.md').write_text('extracted integration prompt\n',encoding='utf-8')
        (attempt/'generation-capsule.md').write_text('extracted integration capsule\n',encoding='utf-8')
        calls=work/'provider-calls.txt'
        env={
            'VERGERAIL_CODEX_PACKAGE':str(run/'runtime'),
            'VERGERAIL_MODEL':'gpt-5.6-luna',
            'VERGERAIL_WORKSPACE':str(run/'workspace'),
            'FAKE_PNG_PATH':str(root/'fixtures/opaque.png'),
            'FAKE_CALLS_PATH':str(calls),
        }
        output=run_bounded([
            PYTHON,str(cli),'execute-scene-provider',str(run),'TASK-extracted','1',
            '--provider-bin',str(provider),'--quality','low',
            '--perfectpixel-bin',str(root/'bin/perfectpixel'),
            '--component-lock',str(lock_path),
        ],cwd=root,timeout=120,extra_env=env)
        try:
            result=json.loads(output)
        except json.JSONDecodeError as error:
            raise GateError(f'EXTRACTED_INTEGRATION_JSON:{output}') from error
        if result.get('status')!='READY_FOR_VISUAL_REVIEW':
            raise GateError(f'EXTRACTED_INTEGRATION_STATUS:{result}')
        ad=attempt
        receipt=json.loads((ad/'execution-receipt.json').read_text(encoding='utf-8'))
        review=json.loads((ad/'mechanical-review.json').read_text(encoding='utf-8'))
        inspection=json.loads((ad/'perfectpixel-inspection.json').read_text(encoding='utf-8'))
        dispatch=json.loads((ad/'scene-provider-dispatch.json').read_text(encoding='utf-8'))
        if review.get('status')!='PASS':
            raise GateError(f'EXTRACTED_INTEGRATION_REVIEW:{review}')
        if calls.read_text(encoding='utf-8').splitlines()!=['image_generate']:
            raise GateError(f'EXTRACTED_INTEGRATION_PROVIDER_CALLS:{calls.read_text(encoding="utf-8")!r}')
        inspection_ref=receipt.get('perfectpixel_inspection')
        if inspection_ref != {'path':'perfectpixel-inspection.json','sha256':sha(ad/'perfectpixel-inspection.json')}:
            raise GateError(f'EXTRACTED_INTEGRATION_INSPECTION_REF:{inspection_ref}')
        output_path=ad/'outputs/generated.png'
        output_sha=sha(output_path)
        inspected=inspection.get('inspection',{})
        if (inspection.get('schema')!='perfectpixel.asset-inspection/1'
                or inspection.get('schema_version')!=1
                or inspection.get('inspection_schema')!='perfectpixel.asset-inspection/1'
                or inspected.get('schema')!='perfectpixel.asset-inspection/1'
                or inspected.get('schemaVersion')!=1
                or inspected.get('ok') is not True):
            raise GateError(f'EXTRACTED_INTEGRATION_INSPECTION_SCHEMA:{inspection}')
        if inspected.get('inputSha256')!=output_sha or inspected.get('inputByteCount')!=output_path.stat().st_size:
            raise GateError(f'EXTRACTED_INTEGRATION_INSPECTION_HASH:{inspection}')
        if inspected.get('width')!=1024 or inspected.get('height')!=1536 or inspected.get('hasAlpha') is not False:
            raise GateError(f'EXTRACTED_INTEGRATION_INSPECTION_PIXELS:{inspection}')
        probe=dispatch.get('provider_protocol_probe',{})
        if probe.get('protocolSchema')!='misegen.provider/2' or probe.get('protocolVersion')!=2 or probe.get('response',{}).get('error',{}).get('code')!='invalid_request' or not probe.get('custody',{}).get('no_survivors'):
            raise GateError(f'EXTRACTED_INTEGRATION_PROVIDER_PROBE:{probe}')
    return 'EXTRACTED_INTEGRATION PASS ready=READY_FOR_VISUAL_REVIEW mechanical=PASS hash_bound_inspection=PASS provider_calls=1'

def _standalone_probe_block(install):
    if '## Verify the pinned components' not in install:
        raise GateError('STANDALONE_INSTALL_PROBE_HEADING')
    section=install.split('## Verify the pinned components',1)[1]
    if '```sh' not in section:
        raise GateError('STANDALONE_INSTALL_PROBE_BLOCK')
    code=section.split('```sh',1)[1]
    if '```' not in code:
        raise GateError('STANDALONE_INSTALL_PROBE_BLOCK_END')
    return code.split('```',1)[0].strip()

def verify_standalone_bundle(archive):
    archive=Path(archive).expanduser().resolve()
    if not archive.is_file() or archive.suffix!='.zip':
        raise GateError('STANDALONE_ARCHIVE_MUST_BE_REGULAR_ZIP')
    with zipfile.ZipFile(archive) as bundle:
        if bundle.testzip() is not None:
            raise GateError('STANDALONE_ZIP_CRC')
        prefix=STANDALONE_PACKAGE_NAME+'/'
        infos=bundle.infolist(); names={x.filename for x in infos}
        required={
            prefix+'INSTALL.md',
            prefix+'SceneAssetCompiler/.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py',
            prefix+'SceneAssetCompiler/contracts/runtime-component-lock.json',
            prefix+'bin/scene-provider-v2',
            prefix+'bin/perfectpixel',
            prefix+'fixtures/transparent.png',
            prefix+'fixtures/opaque.png',
            prefix+'SOURCES/PerfectPixel-0.3.1-source.tar',
            prefix+'SOURCES/PerfectPixel-0.3.1-source.tar.sha256',
            prefix+'SOURCES/PerfectPixel-0.3.1-source.members',
            prefix+'BUNDLE_MANIFEST.sha256',
            prefix+'LICENSES/DEPENDENCY_LICENSE_INVENTORY.md',
            prefix+'LOCKS/scene-provider-v2-probe.json',
            prefix+'LOCKS/perfectpixel-inspection-contract.json',
            prefix+'SceneAssetCompiler/tools/build_standalone.py',
        }
        missing=sorted(required-names)
        if missing: raise GateError(f'STANDALONE_REQUIRED_MISSING:{missing}')
        for info in infos:
            if not info.filename.startswith(prefix):
                raise GateError(f'STANDALONE_ROOT:{info.filename}')
            if '..' in Path(info.filename).parts:
                raise GateError(f'STANDALONE_PATH_ESCAPE:{info.filename}')
            if not info.is_dir() and len(bundle.read(info.filename)) != info.file_size:
                raise GateError(f'STANDALONE_SIZE:{info.filename}')
        regular_names={info.filename for info in infos if not info.is_dir()}
        manifest_name=prefix+'BUNDLE_MANIFEST.sha256'
        manifest_lines=bundle.read(manifest_name).decode('utf-8').splitlines()
        manifest_entries={}
        for line in manifest_lines:
            digest, separator, name=line.partition('  ')
            if separator != '  ' or len(digest) != 64 or any(char not in '0123456789abcdef' for char in digest) or not name:
                raise GateError(f'STANDALONE_MANIFEST_FORMAT:{line}')
            normalized=name[2:] if name.startswith('./') else name
            if not normalized or normalized.startswith('/') or '..' in Path(normalized).parts:
                raise GateError(f'STANDALONE_MANIFEST_PATH:{name}')
            if normalized in manifest_entries:
                raise GateError(f'STANDALONE_MANIFEST_DUPLICATE:{normalized}')
            manifest_entries[normalized]=digest
        expected_manifest_names={name[len(prefix):] for name in regular_names-{manifest_name}}
        if set(manifest_entries) != expected_manifest_names:
            raise GateError(f'STANDALONE_MANIFEST_MEMBERS:{sorted(expected_manifest_names-set(manifest_entries))}:{sorted(set(manifest_entries)-expected_manifest_names)}')
        for name,digest in manifest_entries.items():
            if sha_bytes(bundle.read(prefix+name)) != digest:
                raise GateError(f'STANDALONE_MANIFEST_HASH:{name}')
        install=bundle.read(prefix+'INSTALL.md').decode('utf-8')
        for line in STANDALONE_INSTALL_LINES:
            if line not in install: raise GateError(f'STANDALONE_INSTALL_CONTRACT:{line}')
        for rel in STANDALONE_DOCS:
            doc=bundle.read(prefix+'SceneAssetCompiler/'+rel).decode('utf-8')
            for line in ('BUNDLE_ROOT="$(pwd -P)"','--provider-bin "$PROVIDER_BIN"','--perfectpixel-bin "$PERFECTPIXEL_BIN"','--component-lock "$COMPONENT_LOCK"'):
                if line not in doc: raise GateError(f'STANDALONE_DOC_CONTRACT:{rel}:{line}')
            if '/Users/ax/repoGithub/SMA/scene-asset-compiler' in doc:
                raise GateError(f'STANDALONE_DOC_RETIRED_PATH:{rel}')
        lock=json.loads(bundle.read(prefix+'SceneAssetCompiler/contracts/runtime-component-lock.json'))
        provider=bundle.read(prefix+'bin/scene-provider-v2')
        perfectpixel=bundle.read(prefix+'bin/perfectpixel')
        if lock.get('schemaVersion') != 2:
            raise GateError(f'STANDALONE_COMPONENT_LOCK_VERSION:{lock.get("schemaVersion")}')
        if lock.get('provider',{}).get('name') != 'scene-provider-v2' or lock.get('provider',{}).get('protocolSchema') != 'misegen.provider/2' or lock.get('provider',{}).get('schemaVersion') != 2:
            raise GateError('STANDALONE_PROVIDER_PROTOCOL_LOCK')
        pp_lock=lock.get('perfectpixel',{})
        if pp_lock.get('inspectionSchema') != 'perfectpixel.asset-inspection/1' or pp_lock.get('inspectionSchemaVersion') != 1 or pp_lock.get('role') != 'deterministic-asset-compiler':
            raise GateError('STANDALONE_PERFECTPIXEL_INSPECTION_LOCK')
        if sha_bytes(provider)!=lock['provider']['sha256']:
            raise GateError('STANDALONE_PROVIDER_LOCK')
        if sha_bytes(perfectpixel)!=lock['perfectpixel']['sha256']:
            raise GateError('STANDALONE_PERFECTPIXEL_LOCK')
        for executable_name in ('bin/scene-provider-v2','bin/perfectpixel'):
            executable_info=next((info for info in infos if info.filename == prefix+executable_name), None)
            if executable_info is None or ((executable_info.external_attr >> 16) & 0o111) == 0:
                raise GateError(f'STANDALONE_EXECUTABLE_MODE:{executable_name}')
        source_tar_name=prefix+'SOURCES/PerfectPixel-0.3.1-source.tar'
        source_digest_name=source_tar_name+'.sha256'
        source_digest=bundle.read(source_digest_name).decode('ascii').strip()
        if source_digest != sha_bytes(bundle.read(source_tar_name)):
            raise GateError('STANDALONE_SOURCE_TAR_SHA256')
        source_members_name=prefix+'SOURCES/PerfectPixel-0.3.1-source.members'
        source_members=bundle.read(source_members_name).decode('utf-8').splitlines()
        if len(source_members) != len(set(source_members)) or not source_members:
            raise GateError('STANDALONE_SOURCE_MEMBERS_FORMAT')
        try:
            with tarfile.open(fileobj=io.BytesIO(bundle.read(source_tar_name)), mode='r:*') as source_archive:
                archive_members=sorted(member.name for member in source_archive.getmembers() if member.isfile())
        except (tarfile.TarError, OSError) as error:
            raise GateError('STANDALONE_SOURCE_TAR_FORMAT') from error
        if archive_members != sorted(source_members):
            raise GateError('STANDALONE_SOURCE_MEMBERS_MISMATCH')
        with tempfile.TemporaryDirectory(prefix='scene-asset-compiler-bundle-smoke-') as temp:
            extract_root=Path(temp).resolve()
            for info in infos:
                target=(extract_root/info.filename).resolve()
                if extract_root not in target.parents:
                    raise GateError(f'STANDALONE_EXTRACTION_ESCAPE:{info.filename}')
            bundle.extractall(extract_root)
            # Python's ZipFile.extractall() does not restore Unix executable
            # bits.  The archive records them in external_attr, so restore
            # the recorded regular-file mode before executing the documented
            # INSTALL probe against the extracted bundle.
            for info in infos:
                target=extract_root/info.filename
                mode=(info.external_attr >> 16) & 0o7777
                if target.is_file() and mode:
                    target.chmod(mode)
            root=extract_root/STANDALONE_PACKAGE_NAME
            for executable_name in ('bin/scene-provider-v2','bin/perfectpixel'):
                executable_path=root/executable_name
                if not executable_path.is_file() or not os.access(executable_path, os.X_OK):
                    raise GateError(f'STANDALONE_EXTRACTED_EXECUTABLE:{executable_name}')
            builder=root/'SceneAssetCompiler/tools/build_standalone.py'
            rebuilt=root.parent/'standalone-rebuilt.zip'
            rebuild_output=run_bounded([
                PYTHON,str(builder),'--staging-root',str(root),'--output',str(rebuilt),'--deterministic-check'
            ],cwd=root,timeout=180)
            if 'DETERMINISTIC_REBUILD PASS' not in rebuild_output:
                raise GateError(f'STANDALONE_REBUILD_OUTPUT:{rebuild_output}')
            if sha(rebuilt)!=sha(archive):
                raise GateError(f'STANDALONE_REBUILD_ARCHIVE_MISMATCH:{sha(archive)}:{sha(rebuilt)}')
            extracted_tests=run_bounded(
                [PYTHON,'-m','unittest','discover','-s','tests','-v'],
                cwd=root/'SceneAssetCompiler',timeout=180
            )
            match=re.search(r'Ran (\d+) tests',extracted_tests)
            if match is None or int(match.group(1)) != 105 or not re.search(r'OK \(skipped=1\)',extracted_tests):
                raise GateError(f'STANDALONE_EXTRACTED_TESTS:{extracted_tests[-4000:]}')
            cli=root/'SceneAssetCompiler/.agents/skills/scene-asset-compiler/scripts/scene_asset_compiler.py'
            help_text=run_bounded([PYTHON,str(cli),'execute-scene-provider','--help'],cwd=root)
            for flag in ('--provider-bin PROVIDER_BIN','--quality {auto,low,medium,high}','--perfectpixel-bin PERFECTPIXEL_BIN','--component-lock COMPONENT_LOCK','--dry-run'):
                if flag not in help_text: raise GateError(f'STANDALONE_HELP_FLAG:{flag}')
            root_help=run_bounded([PYTHON,str(cli),'--help'],cwd=root)
            for command in ('execute-scene-provider','probe-scene-provider'):
                if command not in root_help: raise GateError(f'STANDALONE_CANONICAL_COMMAND:{command}')
            for hidden_alias in ('execute-misegen','probe-misegen','execute-vergerail','probe-vergerail'):
                if hidden_alias in root_help: raise GateError(f'STANDALONE_HIDDEN_ALIAS_EXPOSED:{hidden_alias}')
            probe_text=run_bounded(['bash','-eu','-o','pipefail','-c',_standalone_probe_block(install)],cwd=root)
            try: probe=json.loads(probe_text)
            except json.JSONDecodeError as e: raise GateError(f'STANDALONE_PROBE_JSON:{probe_text}') from e
            if (probe.get('status')!='PASS'
                    or probe.get('protocolSchema')!='misegen.provider/2'
                    or probe.get('schemaVersion')!=2
                    or probe.get('protocolVersion')!=2
                    or not probe.get('protocolSchemaEvidence')
                    or probe.get('response',{}).get('schemaVersion')!=2
                    or probe.get('response',{}).get('error',{}).get('code')!='invalid_request'):
                raise GateError(f'STANDALONE_PROBE_PROTOCOL:{probe}')
            if not probe.get('custody',{}).get('no_survivors'):
                raise GateError(f'STANDALONE_PROBE_CUSTODY:{probe}')
            schema_text=run_bounded([str(root/'bin/perfectpixel'),'schema'],cwd=root)
            try: schema=json.loads(schema_text)
            except json.JSONDecodeError as e: raise GateError(f'STANDALONE_PERFECTPIXEL_SCHEMA_JSON:{schema_text}') from e
            if schema.get('cliVersion') != pp_lock.get('cliVersion') or schema.get('inspectSchema') != pp_lock.get('inspectionSchema') or schema.get('role') != pp_lock.get('role') or not {'schema','inspect'}.issubset(schema.get('commands',[])):
                raise GateError(f'STANDALONE_PERFECTPIXEL_SCHEMA:{schema}')
            fixtures=(
                ('fixtures/transparent.png', True, 1236, 1273),
                ('fixtures/opaque.png', False, 1024, 1536),
            )
            for relative_fixture, expected_alpha, expected_width, expected_height in fixtures:
                fixture=root/relative_fixture
                result_text=run_bounded([str(root/'bin/perfectpixel'),'inspect',str(fixture)],cwd=root)
                try: result=json.loads(result_text)
                except json.JSONDecodeError as e: raise GateError(f'STANDALONE_PERFECTPIXEL_INSPECT_JSON:{relative_fixture}') from e
                if result.get('schema') != pp_lock.get('inspectionSchema') or result.get('schemaVersion') != pp_lock.get('inspectionSchemaVersion') or result.get('ok') is not True or result.get('width') != expected_width or result.get('height') != expected_height or result.get('hasAlpha') is not expected_alpha:
                    raise GateError(f'STANDALONE_PERFECTPIXEL_INSPECT:{relative_fixture}:{result}')
            print('DETERMINISTIC_REBUILD PASS command=tools/build_standalone.py archive_sha256='+sha(archive))
            print('EXTRACTED_UNITTEST PASS discovered=105 executed=104 skipped=1')
            print(_extracted_integration_smoke(root, lock))
    print(f'STANDALONE_SMOKE PASS archive={archive} entries={len(infos)} crc=PASS')
    print('INSTALL_PROBE PASS strict_scene_provider_v2 no_external_generation')
    print('EXECUTE_SCENE_PROVIDER_HELP PASS canonical_scene_provider_commands_hidden_compat_aliases_required_provider_quality_perfectpixel_lock_dry_run_flags')
    print('DOC_PATH_CONTRACT PASS absolute_bundle_root_provider_perfectpixel_component_lock')

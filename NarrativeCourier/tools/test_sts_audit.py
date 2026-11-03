import sys
sys.dont_write_bytecode = True
import io
import json
from pathlib import Path
import tempfile
import unittest
import zipfile
from contextlib import redirect_stdout

import sts_audit
import sts_audit_inputs

class AuditTests(unittest.TestCase):
    def test_workflow_is_three_stage_and_blind(self):
        wf=sts_audit.load_workflow()
        self.assertEqual([s['id'] for s in wf['stages']], ['source','screenplay','compare'])
        self.assertIn('synopsis', wf['stages'][1]['blind_to'])
        self.assertIn('raw_screenplay', wf['stages'][2]['blind_to'])

    def test_fountain_input_preserves_source_text(self):
        with tempfile.TemporaryDirectory() as td:
            path=Path(td)/"script.fountain"
            text='EXT. ROAD - NIGHT\n\nMAYA\nGo.\n'
            path.write_text(text,encoding="utf-8")
            got=sts_audit_inputs.capture_input(path)
            self.assertEqual(got.text,text)

    def test_fdx_preserves_paragraph_types(self):
        with tempfile.TemporaryDirectory() as td:
            p=Path(td)/'a.fdx'
            p.write_text('<FinalDraft><Content><Paragraph Type="Scene Heading"><Text>INT. ROOM - DAY</Text></Paragraph><Paragraph Type="Dialogue"><Text>Hello</Text></Paragraph></Content></FinalDraft>')
            got=sts_audit_inputs.extract_fdx(p)
            self.assertIn('[Scene Heading] INT. ROOM - DAY',got)
            self.assertIn('[Dialogue] Hello',got)

    def test_docx_preserves_paragraph_style(self):
        with tempfile.TemporaryDirectory() as td:
            p=Path(td)/'a.docx'
            ns='http://schemas.openxmlformats.org/wordprocessingml/2006/main'
            doc=f'<w:document xmlns:w="{ns}"><w:body><w:p><w:pPr><w:pStyle w:val="SceneHeading"/></w:pPr><w:r><w:t>SCN-001. INT. ROOM - DAY</w:t></w:r></w:p></w:body></w:document>'
            styles=f'<w:styles xmlns:w="{ns}"><w:style w:type="paragraph" w:styleId="SceneHeading"><w:name w:val="Scene Heading"/></w:style></w:styles>'
            with zipfile.ZipFile(p,'w') as z:
                z.writestr('word/document.xml',doc); z.writestr('word/styles.xml',styles)
            got=sts_audit_inputs.extract_docx(p)
            self.assertIn('[Scene Heading] SCN-001.',got)

    def test_bundle_hash_is_deterministic(self):
        wf=sts_audit.load_workflow()
        self.assertEqual(sts_audit.audit_bundle_hash(wf),sts_audit.audit_bundle_hash(wf))

    def test_dry_run_has_no_output_side_effect(self):
        with tempfile.TemporaryDirectory() as td:
            d=Path(td); syn=d/'s.md'; sp=d/'p.fountain'; out=d/'out'
            syn.write_text('A leaves.',encoding='utf-8'); sp.write_text('EXT. ROAD - DAY\n\nA walks away.',encoding='utf-8')
            buf=io.StringIO()
            with redirect_stdout(buf): rc=sts_audit.main(['--synopsis',str(syn),'--screenplay',str(sp),'--out',str(out),'--dry-run'])
            self.assertEqual(rc,0); self.assertFalse(out.exists())
            data=json.loads(buf.getvalue()); self.assertTrue(data['dry_run'])

    def test_fixed_dimensions_reject_missing_axis(self):
        bad={'fidelity':{'dimensions':[]},'integrity':{'dimensions':[]}}
        with self.assertRaises(ValueError): sts_audit.validate_fixed_dimensions(bad)

    def test_alignment_requires_complete_source_coverage(self):
        source={'assertions':[{'id':'SRC-001'}],'relations':[{'id':'REL-001'}],'gaps':[],'conflicts':[]}
        screenplay={'observed_assertions':[{'id':'OBS-001'}],'relations':[{'id':'OREL-001'}]}
        result={'assertion_alignment':[],'relation_alignment':[{'source_id':'REL-001','recovered_ids':['OREL-001']}]}
        with self.assertRaises(ValueError): sts_audit.validate_alignment_coverage(result,source,screenplay)

    def test_alignment_rejects_unknown_recovered_id(self):
        source={'assertions':[{'id':'SRC-001'}],'relations':[],'gaps':[],'conflicts':[]}
        screenplay={'observed_assertions':[{'id':'OBS-001'}],'relations':[]}
        result={'assertion_alignment':[{'source_id':'SRC-001','recovered_ids':['OBS-999']}],'relation_alignment':[]}
        with self.assertRaises(ValueError): sts_audit.validate_alignment_coverage(result,source,screenplay)

    def test_ir_ids_reject_duplicates(self):
        source={'assertions':[{'id':'SRC-001'},{'id':'SRC-001'}],'relations':[],'gaps':[],'conflicts':[]}
        screenplay={'observed_assertions':[],'relations':[]}
        with self.assertRaises(ValueError): sts_audit.validate_ir_ids(source,screenplay)

    def test_verdict_cannot_pass_protected_drift(self):
        source={'assertions':[{'id':'SRC-001','protected':True,'materiality':'CORE'}],'relations':[]}
        result={
            'verdict':'PASS',
            'fidelity':{'dimensions':[{'status':'PASS'}]*8},
            'integrity':{'dimensions':[{'status':'PASS'}]*7},
            'assertion_alignment':[{'source_id':'SRC-001','status':'DRIFTED'}],
            'relation_alignment':[],
            'findings':[]
        }
        with self.assertRaises(ValueError): sts_audit.validate_verdict_coherence(result,source)

    def test_verdict_cannot_pass_uncertainty(self):
        source={'assertions':[{'id':'SRC-001','protected':False,'materiality':'SUPPORT'}],'relations':[]}
        result={
            'verdict':'PASS',
            'fidelity':{'dimensions':[{'status':'PASS'}]*8},
            'integrity':{'dimensions':[{'status':'PASS'}]*7},
            'assertion_alignment':[{'source_id':'SRC-001','status':'UNCERTAIN'}],
            'relation_alignment':[],
            'findings':[]
        }
        with self.assertRaises(ValueError): sts_audit.validate_verdict_coherence(result,source)

    def test_source_schema_requires_world_rule_facets(self):
        import jsonschema
        schema=json.loads((sts_audit.SKILL/'schemas/audit-source-ir.schema.json').read_text())
        value={
            'assertions':[],
            'relations':[{'id':'REL-001','type':'WORLD_RULE','from':'X','to':'Y','materiality':'CORE','evidence':{'quote':'rule'}}],
            'gaps':[],'conflicts':[],
            'source_form':{'function':'NEUTRAL','narrative_function':'none','externalization_items':[]},
            'ending':{'outcome':'x','decisive_actors':[],'semantic_reframe':'','residue':''},
            'open_adaptation_space':[],'confidence':1.0
        }
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(value)

    def test_source_schema_requires_knowledge_acquisition(self):
        import jsonschema
        schema=json.loads((sts_audit.SKILL/'schemas/audit-source-ir.schema.json').read_text())
        value={
            'assertions':[],
            'relations':[{'id':'REL-001','type':'KNOWS','from':'A','to':'secret','materiality':'CORE','evidence':{'quote':'A knows'}}],
            'gaps':[],'conflicts':[],
            'source_form':{'function':'NEUTRAL','narrative_function':'none','externalization_items':[]},
            'ending':{'outcome':'x','decisive_actors':[],'semantic_reframe':'','residue':''},
            'open_adaptation_space':[],'confidence':1.0
        }
        with self.assertRaises(jsonschema.ValidationError):
            jsonschema.Draft202012Validator(schema).validate(value)

if __name__=='__main__': unittest.main()

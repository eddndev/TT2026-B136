from pathlib import Path
from hashlib import sha256
from uuid import UUID
import json, struct, subprocess

u32=lambda value:struct.pack('>I',value)
uid=lambda value:UUID(value).bytes
text=lambda value:u32(len(value.encode('utf-8')))+value.encode('utf-8')
def optional(value):return b'\0' if value is None else b'\1'+text(value)
def declared(value,encoder=text):
    if 'known' in value:return b'\0'+encoder(value['known'])
    return b'\1'+text(value['unknown'])
def locator(value):
    return uid(value['document_id'])+u32(value['version'])+bytes.fromhex(value['digest'])+text(value['locator'])
def license_value(value):return text(value['number'])+text(value['issuer'])
def subject(value):
    if value['kind']=='natural_person':
        name=value['name']
        named=(b'\0'+text(name['known'])) if 'known' in name else b'\1'+text(name['label'])+text(name['reason'])
        fields=named+declared(value['curp'])
        kind=0
    else:
        kind=1;fields=text(value['name'])+declared(value['institutional_identifier'])
    return b'SUBJ1'+bytes([kind])+fields+locator(value['identity_support'])
kind_order=['defendant','victim','defense_counsel','prosecutor','victim_counsel','control_judge','trial_court','expert','police','precautionary_supervisor','other']
def contact(value):
    if 'documented' in value:return b'\2'+locator(value['documented'])
    if 'unknown' in value:return b'\0'+text(value['unknown'])
    return b'\1'+text(value['none'])
def role_fields(value):
    kind=value['kind']
    if kind=='defendant':return declared(value['custody'],lambda v:bytes([{'at_liberty':0,'detained':1}[v]]))
    if kind=='victim':return contact(value['contact'])+contact(value['protection'])
    if kind=='defense_counsel':return license_value(value['license'])+bytes([{'private':0,'public':1}[value['mode']]])
    if kind=='prosecutor':return declared(value['office_identifier'])+declared(value['unit'])+declared(value['license'],license_value)
    if kind=='victim_counsel':return text(value['institution'])+declared(value['license'],license_value)
    if kind=='control_judge':return text(value['court'])
    if kind=='trial_court':return text(value['judicial_district'])+bytes([{'single':0,'collegiate':1}[value['composition']]])
    if kind=='expert':return declared(value['specialty'])+declared(value['license'],license_value)
    if kind=='police':return declared(value['agency'])+declared(value['unit'])
    if kind=='precautionary_supervisor':return declared(value['authority'])+declared(value['unit'])
    if kind=='other':return text(value['label'])+declared(value['description'])
    raise AssertionError(kind)
def participant(value):
    ref=value['subject'];p=value['profile']
    return b'PART2'+uid(ref['id'])+u32(ref['revision'])+bytes.fromhex(ref['digest'])+bytes([{'active':0,'archived':1}[value['directory_status']]])+optional(value['organization'])+optional(value['legal_status'])+bytes([kind_order.index(p['kind'])])+role_fields(p)+locator(value['role_support'])
def declaration(value):
    return b'PCRED1'+b'\0\0'+uid(value['deployment_id'])+bytes.fromhex(value['root_fingerprint'])+uid(value['case_id'])+uid(value['subject_id'])+bytes([value['subject_operation']])+u32(value['expected_subject_revision'])+u32(value['proposed_subject_revision'])+bytes.fromhex(value['subject_digest'])+uid(value['participant_id'])+u32(value['expected_participant_revision'])+u32(value['proposed_participant_revision'])+bytes([value['role_tag']])+bytes.fromhex(value['participant_digest'])+bytes.fromhex(value['certificate_fingerprint'])
def vector(name,value,encoded):
    digest=sha256(encoded).digest()
    external=subprocess.run(['openssl','dgst','-sha256','-binary'],input=encoded,capture_output=True,check=True).stdout
    assert external==digest,name
    return {'name':name,'input':value,'bytes':len(encoded),'hex':encoded.hex(),'sha256':digest.hex()}
proof={'document_id':'33333333-3333-4333-8333-333333333333','version':2,'digest':'11'*32,'locator':'page 1'}
person={'kind':'natural_person','name':{'known':'Ana'},'curp':{'unknown':'Not provided'},'identity_support':proof}
body={'kind':'institutional_body','name':'Trial Court','institutional_identifier':{'known':'COURT-01'},'identity_support':proof}
subject_ref={'id':'11111111-1111-4111-8111-111111111111','revision':2,'digest':sha256(subject(person)).hexdigest()}
lic={'number':'001234','issuer':'Professional Registry'}
profiles=[
 {'kind':'defendant','custody':{'known':'at_liberty'}},
 {'kind':'victim','contact':{'documented':proof},'protection':{'none':'None declared by staff'}},
 {'kind':'defense_counsel','license':lic,'mode':'private'},
 {'kind':'prosecutor','office_identifier':{'known':'OFF-7'},'unit':{'known':'Unit A'},'license':{'known':lic}},
 {'kind':'victim_counsel','institution':'Legal Assistance','license':{'unknown':'Awaiting record'}},
 {'kind':'control_judge','court':'Control Court A'},
 {'kind':'trial_court','judicial_district':'District A','composition':'collegiate'},
 {'kind':'expert','specialty':{'known':'Accounting'},'license':{'unknown':'Awaiting record'}},
 {'kind':'police','agency':{'known':'Agency A'},'unit':{'unknown':'Awaiting record'}},
 {'kind':'precautionary_supervisor','authority':{'known':'Authority A'},'unit':{'known':'Unit A'}},
 {'kind':'other','label':'Interpreter','description':{'known':'Language assistance'}},
]
rows=[vector('natural_person',person,subject(person)),vector('institutional_body',body,subject(body))]
for profile in profiles:
    ref=subject_ref
    if profile['kind']=='trial_court':ref={**ref,'id':'22222222-2222-4222-8222-222222222222','digest':sha256(subject(body)).hexdigest()}
    values={'subject':ref,'directory_status':'active','organization':None,'legal_status':'Declared','profile':profile,'role_support':proof}
    rows.append(vector('role_'+profile['kind'],values,participant(values)))
wide=chr(0x1f642)
max_proof={**proof,'locator':wide*200}
max_person={'kind':'natural_person','name':{'label':wide*200,'reason':wide*500},'curp':{'unknown':wide*500},'identity_support':max_proof}
max_role={'subject':subject_ref,'directory_status':'archived','organization':wide*200,'legal_status':wide*160,'profile':{'kind':'prosecutor','office_identifier':{'unknown':wide*500},'unit':{'unknown':wide*500},'license':{'unknown':wide*500}},'role_support':max_proof}
assert len(subject(max_person))==5676
assert len(participant(max_role))==8380
rows.extend([vector('subject_maximum',max_person,subject(max_person)),vector('participant_maximum',max_role,participant(max_role))])
cred={'deployment_id':'55555555-5555-4555-8555-555555555555','root_fingerprint':'22'*32,'case_id':'66666666-6666-4666-8666-666666666666','subject_id':subject_ref['id'],'subject_operation':0,'expected_subject_revision':2,'proposed_subject_revision':2,'subject_digest':subject_ref['digest'],'participant_id':'77777777-7777-4777-8777-777777777777','expected_participant_revision':4,'proposed_participant_revision':5,'role_tag':5,'participant_digest':rows[7]['sha256'],'certificate_fingerprint':'44'*32}
assert rows[7]['name']=='role_control_judge'
assert len(declaration(cred))==218
rows.append(vector('credential_keep_subject',cred,declaration(cred)))
new_values={**rows[7]['input'],'subject':{**subject_ref,'revision':1}}
created={**cred,'subject_operation':1,'expected_subject_revision':0,'proposed_subject_revision':1,'expected_participant_revision':0,'proposed_participant_revision':1,'participant_digest':sha256(participant(new_values)).hexdigest()}
rows.append(vector('credential_new_subject',created,declaration(created)))
Path(__file__).with_name('typed_participant_vectors.json').write_text(json.dumps(rows,indent=2,ensure_ascii=True)+'\n')
print(json.dumps([{'name':row['name'],'bytes':row['bytes'],'sha256':row['sha256']} for row in rows],indent=2))

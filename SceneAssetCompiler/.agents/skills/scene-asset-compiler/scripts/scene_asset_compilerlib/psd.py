from __future__ import annotations
import struct, zlib

class PsdError(ValueError): pass
class R:
    def __init__(self,b): self.b=memoryview(b); self.o=0
    def need(self,n):
        if self.o+n>len(self.b): raise PsdError('PSD_TRUNCATED')
    def read(self,n): self.need(n); x=self.b[self.o:self.o+n].tobytes(); self.o+=n; return x
    def u16(self): return struct.unpack('>H',self.read(2))[0]
    def i16(self): return struct.unpack('>h',self.read(2))[0]
    def u32(self): return struct.unpack('>I',self.read(4))[0]
    def i32(self): return struct.unpack('>i',self.read(4))[0]
    def u64(self): return struct.unpack('>Q',self.read(8))[0]
    def sub(self,n): return R(self.read(n))
    def skip(self,n): self.read(n)

def pascal(r,pad=4):
    n=r.read(1)[0]; s=r.read(n).decode('latin1','replace'); total=1+n; p=(-total)%pad
    if p:r.skip(p)
    return s

def pascal4(r): return pascal(r,4)
def unicode_name(r):
    try:
        n=r.u32(); return ''.join(chr(r.u16()) for _ in range(n))
    except Exception:return None

def _packbits_row(data, expected):
    out=bytearray(); i=0
    while i<len(data):
        n=data[i]; i+=1
        if n<=127:
            count=n+1
            if i+count>len(data): return False
            out.extend(data[i:i+count]); i+=count
        elif n>=129:
            if i>=len(data): return False
            out.extend(bytes([data[i]])*(257-n)); i+=1
        if len(out)>expected: return False
    return len(out)==expected

def _validate_channel_payload(data,compression,width,height,depth,version=1):
    if width<=0 or height<=0 or depth not in (1,8,16,32): return False
    row_bytes=(width*depth+7)//8; expected=row_bytes*height
    if compression==0: return len(data)==expected
    if compression==1:
        count_size=2 if version==1 else 4; table_size=height*count_size
        if len(data)<table_size: return False
        fmt='>H' if count_size==2 else '>I'; lengths=[struct.unpack(fmt,data[i*count_size:(i+1)*count_size])[0] for i in range(height)]
        if table_size+sum(lengths)!=len(data): return False
        offset=table_size
        for length in lengths:
            row=data[offset:offset+length]; offset+=length
            if not _packbits_row(row,row_bytes): return False
        return True
    if compression in (2,3):
        try: raw=zlib.decompress(data)
        except zlib.error: return False
        return len(raw)==expected
    return False

def _path_structure(payload):
    if not payload or len(payload)%26: return False,'record-alignment',[]
    records=[payload[i:i+26] for i in range(0,len(payload),26)]
    selectors=[struct.unpack('>H',r[:2])[0] for r in records]
    if selectors[0]!=6: return False,'first-record-not-fill-rule',selectors
    i=1; subpaths=0
    while i<len(records):
        selector=selectors[i]
        if selector in (6,8): i+=1; continue
        if selector not in (0,3): return False,f'orphan-selector-{selector}-at-{i}',selectors
        if selector!=0: return False,f'open-subpath-at-{i}',selectors
        count=struct.unpack('>H',records[i][2:4])[0]
        if count<3: return False,f'degenerate-subpath-at-{i}',selectors
        allowed=(1,2) if selector==0 else (4,5)
        if i+count>=len(records): return False,f'subpath-overrun-at-{i}',selectors
        knots=selectors[i+1:i+1+count]
        if any(x not in allowed for x in knots): return False,f'subpath-knot-type-at-{i}',selectors
        anchors=[]
        for record in records[i+1:i+1+count]:
            y=struct.unpack('>i',record[10:14])[0]/16777216.0; x=struct.unpack('>i',record[14:18])[0]/16777216.0
            if not (0<=x<=1 and 0<=y<=1): return False,f'anchor-outside-canvas-at-{i}',selectors
            anchors.append((x,y))
        if len(set(anchors))<3: return False,f'repeated-anchor-at-{i}',selectors
        area=abs(sum(anchors[j][0]*anchors[(j+1)%len(anchors)][1]-anchors[(j+1)%len(anchors)][0]*anchors[j][1] for j in range(len(anchors))))/2
        if area<=1e-10: return False,f'zero-area-subpath-at-{i}',selectors
        subpaths+=1; i+=count+1
    if not subpaths: return False,'no-subpath',selectors
    return True,None,selectors

def _icc_profile_summary(payload):
    declared=struct.unpack('>I',payload[:4])[0] if len(payload)>=4 else None
    valid=bool(len(payload)>=128 and payload[36:40]==b'acsp' and declared is not None and 128<=declared<=len(payload))
    summary={'declared_size':declared,'header_signature':payload[36:40].decode('latin1') if len(payload)>=40 else None,'valid_header':valid}
    if not valid: return summary
    summary.update({'profile_class':payload[12:16].decode('latin1'),'color_space':payload[16:20].decode('latin1'),'pcs':payload[20:24].decode('latin1')})
    descriptions=[]; tags={}
    try:
        count=struct.unpack('>I',payload[128:132])[0]
        if 132+count*12>declared: raise ValueError('tag table')
        for i in range(count):
            base=132+i*12; sig=payload[base:base+4]; off,size=struct.unpack('>II',payload[base+4:base+12])
            if off+size>declared or size<8: raise ValueError('tag bounds')
            data=payload[off:off+size]
            tags[sig]=data
            if sig==b'desc' and data[:4]==b'desc' and len(data)>=12:
                n=struct.unpack('>I',data[8:12])[0]
                if n and 12+n<=len(data): descriptions.append(data[12:12+n-1].decode('latin1','replace'))
            elif sig==b'desc' and data[:4]==b'mluc' and len(data)>=28:
                n,record_size=struct.unpack('>II',data[8:16])
                if n and record_size>=12 and 16+record_size<=len(data):
                    length,offset=struct.unpack('>II',data[20:28])
                    if offset+length<=len(data): descriptions.append(data[offset:offset+length].decode('utf-16-be','replace'))
    except (ValueError,struct.error):
        summary['valid_header']=False; summary['tag_table_valid']=False
        return summary
    summary['tag_table_valid']=True; summary['description']=' | '.join(descriptions); summary['tags']=sorted(x.decode('latin1') for x in tags)
    desc=summary['description'].lower().replace(' ','')
    def fixed(data,offset): return struct.unpack('>i',data[offset:offset+4])[0]/65536.0
    expected_xyz={b'rXYZ':(0.4361,0.2225,0.0139),b'gXYZ':(0.3851,0.7169,0.0971),b'bXYZ':(0.1431,0.0606,0.7142)}
    xyz_ok=True
    for sig,expected in expected_xyz.items():
        data=tags.get(sig,b'')
        if len(data)<20 or data[:4]!=b'XYZ ' or any(abs(fixed(data,8+j*4)-expected[j])>0.03 for j in range(3)): xyz_ok=False
    def trc_ok(data):
        if len(data)<12: return False
        if data[:4]==b'para':
            fn=struct.unpack('>H',data[8:10])[0]; counts={0:1,1:3,2:4,3:5,4:7}
            if fn not in counts or len(data)<12+counts[fn]*4: return False
            values=[fixed(data,12+i*4) for i in range(counts[fn])]
            if fn==4:
                target=(2.4,1/1.055,0.055/1.055,1/12.92,0.04045,0.0,0.0)
                return all(abs(a-b)<=0.025 for a,b in zip(values,target))
            return False
        if data[:4]==b'curv':
            count=struct.unpack('>I',data[8:12])[0]
            if count<16 or len(data)<12+count*2: return False
            values=struct.unpack(f'>{count}H',data[12:12+count*2])
            for index in (1,count//8,count//4,count//2,3*count//4,count-1):
                x=index/(count-1); expected=x/12.92 if x<=0.04045 else ((x+0.055)/1.055)**2.4
                if abs(values[index]/65535-expected)>0.025: return False
            return True
        return False
    trc_valid=all(trc_ok(tags.get(sig,b'')) for sig in (b'rTRC',b'gTRC',b'bTRC'))
    summary['srgb_primaries_valid']=xyz_ok; summary['srgb_trc_valid']=trc_valid
    summary['is_srgb']=summary['profile_class'] in ('mntr','scnr') and summary['color_space']=='RGB ' and summary['pcs']=='XYZ ' and 'srgb' in desc and xyz_ok and trc_valid
    return summary

def _parse_resources(data:bytes):
    rr=R(data); out=[]; saved=[]; clipping=None; work=None; icc=None; icc_untagged=False
    while rr.o+12<=len(rr.b):
        sig=rr.read(4)
        if sig!=b'8BIM': break
        rid=rr.u16(); name=pascal(rr,2); ln=rr.u32(); payload=rr.read(ln)
        if ln%2 and rr.o<len(rr.b): rr.skip(1)
        item={'id':rid,'name':name,'length':ln}; out.append(item)
        if rid==1039:
            icc={'id':rid,'length':ln,'present':True,**_icc_profile_summary(payload)}
        elif rid==1041:
            icc_untagged=bool(payload and payload[0])
        elif rid==1025:
            valid,error,selectors=_path_structure(payload)
            work={'id':rid,'records':ln//26,'valid_record_alignment':ln%26==0,'valid_path_structure':valid,'sequence_error':error}
        elif 2000<=rid<=2997:
            valid,error,selectors=_path_structure(payload)
            saved.append({'id':rid,'name':name,'records':ln//26,'valid_record_alignment':ln%26==0,'selectors':selectors,'valid_path_structure':valid,'sequence_error':error})
        elif rid==2999:
            try:
                pr=R(payload); n=pr.read(1)[0]; pname=pr.read(n).decode('ascii','replace'); flat=struct.unpack('>i',pr.read(4))[0]/65536.0; rule=pr.u16()
                clipping={'path_name':pname,'flatness':flat,'fill_rule':rule}
            except Exception:
                clipping={'parse_error':True}
    return {'all':out,'saved_paths':saved,'clipping_path':clipping,'working_path':work,'icc_profile':icc,'icc_untagged':icc_untagged}

def parse_psd(data:bytes):
    r=R(data)
    if r.read(4)!=b'8BPS': raise PsdError('PSD_BAD_SIGNATURE')
    version=r.u16()
    if version not in (1,2): raise PsdError('PSD_BAD_VERSION')
    r.skip(6); channels=r.u16(); height=r.u32(); width=r.u32(); depth=r.u16(); color_mode=r.u16()
    r.skip(r.u32()); ir_len=r.u32(); resources=_parse_resources(r.read(ir_len))
    lm_len=r.u32() if version==1 else r.u64(); layers=[]
    if lm_len:
      lm=r.sub(lm_len); li_len=lm.u32() if version==1 else lm.u64()
      if li_len:
        li=lm.sub(li_len); count=abs(li.i16())
        for _ in range(count):
          top,left,bottom,right=li.i32(),li.i32(),li.i32(),li.i32(); cc=li.u16(); ch=[]; channel_lengths=[]
          for __ in range(cc):
            ch.append(li.i16()); channel_lengths.append(li.u32() if version==1 else li.u64())
          if li.read(4)!=b'8BIM': raise PsdError('PSD_BAD_BLEND_SIGNATURE')
          li.skip(8); extra=li.sub(li.u32()); mask_len=extra.u32(); extra.skip(mask_len); extra.skip(extra.u32()); name=pascal4(extra); uni=None; vector_mask=False; section_type=0; tagged_keys=[]
          while extra.o+12<=len(extra.b):
            sig=extra.read(4); key=extra.read(4); ln=extra.u32()
            if extra.o+ln>len(extra.b): break
            block=extra.sub(ln)
            tagged_keys.append(key.decode('latin1','replace'))
            if key==b'luni': uni=unicode_name(block)
            if key in (b'vmsk',b'vsms'): vector_mask=True
            if key in (b'lsct',b'lsdk') and len(block.b)>=4: section_type=block.u32()
            if ln%2 and extra.o<len(extra.b): extra.skip(1)
            if sig not in (b'8BIM',b'8B64'): break
          smart_keys={'SoLd','SoLE','PlLd','lnk2','lnk3','lnkD','lnkE'}
          nonpixel_keys={'TySh','SoCo','GdFl','PtFl','brit','levl','curv','expA','vibA','hue2','hue ','blnc','blwh','phfl','mixr','clrL','nvrt','post','thrs','grdm','selc'}
          layers.append({'name':uni or name,'bounds':[left,top,right,bottom],'channel_ids':ch,'channel_lengths':channel_lengths,'user_mask_bytes':mask_len,'has_user_pixel_mask':mask_len>0 or -2 in ch or -3 in ch,'has_vector_mask':vector_mask,'section_divider_type':section_type,'tagged_keys':tagged_keys,'has_smart_object':any(k in smart_keys for k in tagged_keys),'has_nonpixel_content':any(k in nonpixel_keys for k in tagged_keys)})
        for layer in layers:
          payload_ok=True; color_payload=False; compressions=[]
          left,top,right,bottom=layer['bounds']; width=max(0,right-left); height=max(0,bottom-top)
          for channel_id,length in zip(layer['channel_ids'],layer['channel_lengths']):
            if length<2 or length>len(li.b)-li.o: raise PsdError('PSD_LAYER_CHANNEL_DATA')
            channel=li.sub(length); compression=channel.u16(); compressions.append(compression)
            strict=channel_id>=-1
            valid=compression in (0,1,2,3) and (not strict or _validate_channel_payload(channel.read(len(channel.b)-channel.o),compression,width,height,depth,version))
            if not valid: raise PsdError('PSD_LAYER_CHANNEL_DATA')
            if channel_id>=0 and valid: color_payload=True
          layer['channel_payload_valid']=payload_ok
          layer['channel_compressions']=compressions
          layer['is_pixel_leaf']=layer['section_divider_type']==0 and not layer['has_nonpixel_content'] and payload_ok and color_payload
    return {'version':version,'channels':channels,'width':width,'height':height,'depth':depth,'color_mode':color_mode,'layers':layers,'image_resources':resources}

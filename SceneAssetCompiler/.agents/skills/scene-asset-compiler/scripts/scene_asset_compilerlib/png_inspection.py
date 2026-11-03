"""Bounded structural PNG parsing for SAC review and provider boundaries."""
from __future__ import annotations

import struct
import zlib

MAX_PNG_DECODED_BYTES = 64 * 1024 * 1024

class PngError(ValueError): pass

def _paeth(a,b,c):
    p=a+b-c; pa=abs(p-a); pb=abs(p-b); pc=abs(p-c)
    return a if pa<=pb and pa<=pc else b if pb<=pc else c

def inspect_png(data):
    """Parse a non-interlaced PNG and report whether any encoded pixel is transparent."""
    if len(data)<33 or data[:8]!=b'\x89PNG\r\n\x1a\n': raise PngError('PNG_BAD_SIGNATURE')
    offset=8; chunks=[]; idat=[]; ihdr=None; trns=None; plte=None; seen_iend=False
    while offset<len(data):
        if offset+12>len(data): raise PngError('PNG_TRUNCATED_CHUNK')
        length=struct.unpack('>I',data[offset:offset+4])[0]; kind=data[offset+4:offset+8]
        end=offset+12+length
        if end>len(data): raise PngError('PNG_TRUNCATED_CHUNK')
        payload=data[offset+8:offset+8+length]
        expected=struct.unpack('>I',data[offset+8+length:end])[0]
        if zlib.crc32(kind+payload)&0xffffffff != expected: raise PngError(f'PNG_BAD_CRC:{kind.decode("latin1")}')
        if not chunks and kind!=b'IHDR': raise PngError('PNG_IHDR_NOT_FIRST')
        chunks.append(kind)
        if kind==b'IHDR':
            if ihdr is not None or length!=13: raise PngError('PNG_BAD_IHDR')
            ihdr=struct.unpack('>IIBBBBB',payload)
        elif kind==b'IDAT': idat.append(payload)
        elif kind==b'tRNS': trns=payload
        elif kind==b'PLTE': plte=payload
        elif kind==b'IEND':
            if length: raise PngError('PNG_BAD_IEND')
            seen_iend=True; offset=end; break
        offset=end
    if not seen_iend or offset!=len(data): raise PngError('PNG_BAD_TERMINATION')
    if ihdr is None or not idat: raise PngError('PNG_REQUIRED_CHUNK_MISSING')
    width,height,depth,color_type,compression,filter_method,interlace=ihdr
    if not width or not height: raise PngError('PNG_EMPTY_CANVAS')
    allowed={0:{1,2,4,8,16},2:{8,16},3:{1,2,4,8},4:{8,16},6:{8,16}}
    if color_type not in allowed or depth not in allowed[color_type]: raise PngError('PNG_UNSUPPORTED_COLOR_TYPE')
    if color_type==3:
        if plte is None or not plte or len(plte)%3 or len(plte)>768: raise PngError('PNG_BAD_PALETTE')
        if len(plte)//3 > (1<<depth): raise PngError('PNG_PALETTE_TOO_LARGE')
        if trns is not None and len(trns)>len(plte)//3: raise PngError('PNG_BAD_TRNS')
    elif trns is not None and ((color_type==0 and len(trns)!=2) or (color_type==2 and len(trns)!=6) or color_type in (4,6)):
        raise PngError('PNG_BAD_TRNS')
    if compression!=0 or filter_method!=0: raise PngError('PNG_UNSUPPORTED_METHOD')
    if interlace!=0: raise PngError('PNG_INTERLACE_UNSUPPORTED')
    channels={0:1,2:3,3:1,4:2,6:4}[color_type]
    row_bytes=(width*channels*depth+7)//8; bpp=max(1,(channels*depth+7)//8)
    expected_raw=height*(row_bytes+1)
    if expected_raw>MAX_PNG_DECODED_BYTES: raise PngError('PNG_DECODE_LIMIT')
    try:
        inflater=zlib.decompressobj()
        raw=inflater.decompress(b''.join(idat),expected_raw+1)
    except zlib.error as e: raise PngError(f'PNG_DEFLATE:{e}') from e
    if len(raw)>expected_raw or inflater.unconsumed_tail: raise PngError('PNG_DECODE_LIMIT')
    if not inflater.eof or inflater.unused_data or len(raw)!=expected_raw: raise PngError('PNG_SCANLINE_SIZE')
    rows=[]; prior=bytearray(row_bytes); pos=0
    for _ in range(height):
        f=raw[pos]; pos+=1; cur=bytearray(raw[pos:pos+row_bytes]); pos+=row_bytes
        if f>4: raise PngError(f'PNG_FILTER:{f}')
        for i,x in enumerate(cur):
            left=cur[i-bpp] if i>=bpp else 0; up=prior[i]; upper_left=prior[i-bpp] if i>=bpp else 0
            if f==1: cur[i]=(x+left)&255
            elif f==2: cur[i]=(x+up)&255
            elif f==3: cur[i]=(x+((left+up)//2))&255
            elif f==4: cur[i]=(x+_paeth(left,up,upper_left))&255
        rows.append(bytes(cur)); prior=cur
    alpha=False; alpha_min=None; alpha_max=None; transparent_pixels=0; fully_transparent_pixels=0; near_opaque_pixels=0; alpha_bbox=None
    def observe(value,x,y,opaque):
        nonlocal alpha,alpha_min,alpha_max,transparent_pixels,fully_transparent_pixels,near_opaque_pixels,alpha_bbox
        alpha_min=value if alpha_min is None else min(alpha_min,value); alpha_max=value if alpha_max is None else max(alpha_max,value)
        if value<opaque: alpha=True; transparent_pixels+=1
        if value==0: fully_transparent_pixels+=1
        if value>=opaque-(opaque//50): near_opaque_pixels+=1
        if value>0:
            alpha_bbox=(x,y,x+1,y+1) if alpha_bbox is None else (min(alpha_bbox[0],x),min(alpha_bbox[1],y),max(alpha_bbox[2],x+1),max(alpha_bbox[3],y+1))
    if color_type in (4,6):
        sample_bytes=depth//8; pixel_bytes=channels*sample_bytes; alpha_offset=(channels-1)*sample_bytes
        opaque=(1<<(8*sample_bytes))-1
        for y,row in enumerate(rows):
            for x,i in enumerate(range(0,len(row),pixel_bytes)):
                observe(int.from_bytes(row[i+alpha_offset:i+alpha_offset+sample_bytes],'big'),x,y,opaque)
    elif color_type==3:
        entries=len(plte)//3; mask=(1<<depth)-1
        for y,row in enumerate(rows):
            for x in range(width):
                bit=x*depth; index=(row[bit//8]>>(8-depth-(bit%8)))&mask
                if index>=entries: raise PngError('PNG_PALETTE_INDEX_OUT_OF_RANGE')
                observe(trns[index] if trns is not None and index<len(trns) else 255,x,y,255)
    elif color_type==0 and trns is not None:
        transparent=struct.unpack('>H',trns)[0]; opaque=(1<<depth)-1; mask=opaque
        if transparent>opaque: raise PngError('PNG_BAD_TRNS_SAMPLE')
        for y,row in enumerate(rows):
            for x in range(width):
                sample=int.from_bytes(row[x*2:x*2+2],'big') if depth==16 else (row[x] if depth==8 else (row[(x*depth)//8]>>(8-depth-((x*depth)%8)))&mask)
                observe(0 if sample==transparent else opaque,x,y,opaque)
    elif color_type==2 and trns is not None:
        transparent=struct.unpack('>HHH',trns); sample_bytes=depth//8; opaque=(1<<depth)-1
        if any(x>opaque for x in transparent): raise PngError('PNG_BAD_TRNS_SAMPLE')
        for y,row in enumerate(rows):
            for x,i in enumerate(range(0,len(row),3*sample_bytes)):
                pixel=tuple(int.from_bytes(row[i+j*sample_bytes:i+(j+1)*sample_bytes],'big') for j in range(3))
                observe(0 if pixel==transparent else opaque,x,y,opaque)
    total=width*height
    return {'width':width,'height':height,'bit_depth':depth,'color_type':color_type,'has_transparent_pixels':alpha,'alpha_min':alpha_min,'alpha_max':alpha_max,'transparent_pixels':transparent_pixels,'fully_transparent_pixels':fully_transparent_pixels,'near_opaque_pixels':near_opaque_pixels,'nonzero_pixels':(total-fully_transparent_pixels if alpha_min is not None else None),'alpha_bbox':alpha_bbox}

def png_alpha(data):
    try: return inspect_png(data)['has_transparent_pixels']
    except PngError: return False

#!/usr/bin/env python3
from pathlib import Path
import json, shutil, subprocess, sys, tempfile

def run(cmd, timeout=30):
    return subprocess.run(cmd,check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,timeout=timeout)
def need(name):
    p=shutil.which(name)
    if not p: raise RuntimeError(f'{name} unavailable')
    return p

try:
    magick=need('magick'); weasy=need('weasyprint'); ffmpeg=need('ffmpeg'); ffprobe=need('ffprobe')
    with tempfile.TemporaryDirectory(prefix='narrative-courier-render-smoke-') as td:
        d=Path(td); html=d/'page.html'; pdf=d/'page.pdf'; svg=d/'page.svg'; png=d/'page.png'; webp=d/'page.webp'; frames=d/'frames'; mp4=d/'six.mp4'
        html.write_text('<!doctype html><meta charset="utf-8"><pre>SCN-001. TEST\nExact narrative projection.</pre>',encoding='utf-8')
        run([weasy,str(html),str(pdf)],60)
        assert pdf.read_bytes().startswith(b'%PDF-')
        svg.write_text('<svg xmlns="http://www.w3.org/2000/svg" width="640" height="360"><rect width="100%" height="100%" fill="white"/><text x="40" y="80" font-size="24">SCN-001. TEST</text></svg>',encoding='utf-8')
        run([magick,str(svg),str(png)],60)
        dims=run([magick,'identify','-format','%w %h',str(png)]).stdout.strip(); assert dims=='640 360',dims
        run([magick,str(svg),str(webp)],60)
        webp_dims=run([magick,'identify','-format','%w %h',str(webp)]).stdout.strip(); assert webp_dims=='640 360',webp_dims
        frames.mkdir()
        for i in range(1,7):
            target=frames/f'frame{i:06}.png'
            try: target.hardlink_to(png)
            except OSError: shutil.copy2(png,target)
        run([ffmpeg,'-y','-v','error','-framerate','6/1','-i',str(frames/'frame%06d.png'),'-frames:v','6','-c:v','libx264','-pix_fmt','yuv420p',str(mp4)],120)
        probe=json.loads(run([ffprobe,'-v','error','-count_frames','-select_streams','v:0','-show_entries','stream=width,height,nb_read_frames','-of','json',str(mp4)]).stdout)
        stream=probe['streams'][0]; frames_n=int(stream['nb_read_frames']); assert (stream['width'],stream['height'],frames_n)==(640,360,6),stream
        print('PASS HTML source fixture')
        print('PASS PDF WeasyPrint smoke')
        print('PASS PNG ImageMagick 640x360 smoke')
        print('PASS WEBP ImageMagick 640x360 smoke')
        print('PASS MP4 ffmpeg 640x360 exact 6-frame smoke')
except Exception as e:
    print(f'FAIL renderer smoke: {e}',file=sys.stderr); sys.exit(1)

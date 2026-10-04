"""Compare captured pixels without rescaling or per-avatar alignment."""
import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

p=argparse.ArgumentParser()
p.add_argument('--native',type=Path,required=True)
p.add_argument('--reference',type=Path,required=True)
p.add_argument('--out',type=Path,required=True)
a=p.parse_args()
a.out.mkdir(parents=True,exist_ok=True)
manifest=json.loads((a.native/'manifest.json').read_text())
for entry in manifest['files']:
    assert hashlib.sha256((a.native/entry['path']).read_bytes()).hexdigest()==entry['sha256'],entry['path']
bounds=json.loads((a.reference/'bounds.json').read_text())
palette=[[245,244,236],[220,216,196],[18,16,6]]
alignments=[]
for page in bounds:
    if '-square-' not in page['id']:continue
    im=np.asarray(Image.open(a.native/'window'/f"{page['id']}.png").convert('RGB'))
    mask=np.all(im[100:500,32:304]==palette[0],axis=2)
    yy,xx=np.where(mask)
    box=page['bounds'][0]
    assert xx.max()-xx.min()+1==box['width'] and yy.max()-yy.min()+1==box['height'],page['id']
    alignments.append({'page':page['id'],'dx':int(xx.min()+32-box['x']),'dy':int(yy.min()+100-box['y'])})
assert len(alignments)==10
assert len({(r['dx'],r['dy']) for r in alignments})==1,alignments
dx,dy=alignments[0]['dx'],alignments[0]['dy']
rows=[]
shapes=['round','organic','boxy','capsule','nub','cloud','droplet','hexagon','sun','triangle']
for page in bounds:
    name=page['id']
    native=Image.open(a.native/'window'/f'{name}.png').convert('RGB')
    reference=Image.open(a.reference/f'{name}.png').convert('RGB')
    assert native.size==(1500,882) and reference.size==(1500,850)
    host=[244,245,247] if name.endswith('light') else [17,19,24]
    for shape,box in zip(shapes,page['bounds'],strict=True):
        x,y,w,h=[int(box[k]) for k in ('x','y','width','height')]
        guard=3
        assert x+dx>=guard and y+dy>=guard and x+dx+w+guard<=native.width and y+dy+h+guard<=native.height
        nr=np.asarray(native.crop((x+dx-guard,y+dy-guard,x+dx+w+guard,y+dy+h+guard)))
        rr=np.asarray(reference.crop((x-guard,y-guard,x+w+guard,y+h+guard)))
        diff=np.abs(nr.astype(int)-rr.astype(int))
        changed=np.any(diff!=0,axis=2)
        edge=np.zeros(rr.shape[:2],dtype=bool)
        horizontal=np.any(rr[:,1:]!=rr[:,:-1],axis=2)
        vertical=np.any(rr[1:]!=rr[:-1],axis=2)
        edge[:,:-1]|=horizontal;edge[:,1:]|=horizontal
        edge[:-1]|=vertical;edge[1:]|=vertical
        bands=[]
        for distance in (0,1,2):
            expanded=np.asarray(Image.fromarray(edge).filter(ImageFilter.MaxFilter(2*distance+1))) if distance else edge
            bands.append(int((changed & ~expanded).sum()))
        # A three-pixel guard checks spilling independently of the contour metric.
        outer=np.ones(rr.shape[:2],dtype=bool);outer[guard-1:guard+h+1,guard-1:guard+w+1]=False
        spill=int((np.any(nr!=host,axis=2)&outer).sum())
        rows.append({'page':name,'shape':shape,'size':w,'mean_absolute_channel_error':float(diff.mean()),'max_channel_error':int(diff.max()),'changed_pixels':int(changed.sum()),'changed_outside_reference_edge':bands[0],'changed_farther_than_one_pixel':bands[1],'changed_farther_than_two_pixels':bands[2],'spill_beyond_one_pixel_guard':spill})
    pair=Image.new('RGB',(1500,882+850+72),'white');draw=ImageDraw.Draw(pair)
    draw.text((16,8),f'Pinned upstream SVG / {name}',fill='black');pair.paste(reference,(0,28))
    draw.text((16,890),'Native GPUI / macOS / 116d803 / full window',fill='black');pair.paste(native,(0,922))
    pair.save(a.out/f'{name}-pair.png')
assert len(rows)==400
report={'native_commit':manifest['commit'],'comparison':'raw RGB, no scaling; one global translation derived from all 10 square-background captures; UI chrome/fonts excluded from per-avatar metrics','alignment':{'dx':dx,'dy':dy,'anchors':alignments},'case_count':len(rows),'rows':rows,'summary':{'max_mean_absolute_channel_error':max(r['mean_absolute_channel_error'] for r in rows),'pixels_farther_than_one_pixel':sum(r['changed_farther_than_one_pixel'] for r in rows),'pixels_farther_than_two_pixels':sum(r['changed_farther_than_two_pixels'] for r in rows),'spill_pixels':sum(r['spill_beyond_one_pixel_guard'] for r in rows)}}
(a.out/'comparison.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'alignment':[dx,dy],'case_count':len(rows),'summary':report['summary'],'outliers':[r for r in rows if r['changed_farther_than_one_pixel'] or r['spill_beyond_one_pixel_guard']]},indent=2))

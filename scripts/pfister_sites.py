# Finds the Pfister et al. 2009 watershed under each site of data/sites.json and prints its
# attributes (WSI_1 = water stress index, M2YR = ecosystem factor CF_EQ in PDF·m²·yr/m³).
# 1. Download the layer from the Wayback Machine and unzip it to get doc.kml:
#    https://web.archive.org/web/20150910134212/http://www.ifu.ethz.ch/ESD/downloads/EI99plus/Impact_factors_Water_LCA_pfister_et_al.kmz
# 2. python3 -I scripts/pfister_sites.py doc.kml data/sites.json
# The `pfister` block of data/sites.json takes M2YR as cf_eq and M2YR / 0.3514 as multiplier.
import json, re, sys

kml_path, sites_path = sys.argv[1], sys.argv[2]
sites = json.load(open(sites_path))["sites"]
txt = open(kml_path, encoding="utf-8").read()
cut = txt.find("<name>Information Points</name>")
poly_txt = txt[:cut]

pm_re = re.compile(r"<Placemark>(.*?)</Placemark>", re.S)
sd_re = re.compile(r'<SimpleData name="(\w+)">([^<]*)</SimpleData>')
ring_re = re.compile(r"<outerBoundaryIs>.*?<coordinates>(.*?)</coordinates>", re.S)
inner_re = re.compile(r"<innerBoundaryIs>.*?<coordinates>(.*?)</coordinates>", re.S)


def parse_ring(s):
    pts = []
    for tok in s.split():
        x, y = tok.split(",")[:2]
        pts.append((float(x), float(y)))
    return pts


def inside(pt, ring):
    x, y = pt
    c = False
    n = len(ring)
    for i in range(n):
        x1, y1 = ring[i]
        x2, y2 = ring[(i + 1) % n]
        if (y1 > y) != (y2 > y):
            xi = x1 + (y - y1) * (x2 - x1) / (y2 - y1)
            if x < xi:
                c = not c
    return c


out = {s["id"]: {"hits": [], "near": []} for s in sites}
for m in pm_re.finditer(poly_txt):
    body = m.group(1)
    attrs = {k: float(v) for k, v in sd_re.findall(body)}
    outers = [parse_ring(r) for r in ring_re.findall(body)]
    inners = [parse_ring(r) for r in inner_re.findall(body)]
    for s in sites:
        p = (s["lon"], s["lat"])
        if any(inside(p, r) for r in outers) and not any(inside(p, r) for r in inners):
            out[s["id"]]["hits"].append(attrs)
        for r in outers:
            for (x, y) in r:
                if abs(x - p[0]) <= 1.0 and abs(y - p[1]) <= 1.0:
                    out[s["id"]]["near"].append(attrs["WSI_1"])
                    break
for k, v in out.items():
    near = sorted(set(v["near"]))
    print(k, json.dumps(v["hits"]), "near_wsi_range", (near[0], near[-1]) if near else None, len(near))

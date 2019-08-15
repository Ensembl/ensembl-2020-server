from flask import jsonify, Blueprint, request
import yaml, re, time, os.path, string, base64, math, tzlocal, logging
import collections, datetime, copy
import pyBigWig, bbi, png, pytz

from seqcache import SequenceCache

from .source.source import BAISources

from .debug import debug_endpoint
from .data import get_bigbed_data, get_bigwig_data
from .model.universe import Universe
from .model.config import BAIConfig
from .model.leaf import Leaf

breakdown = [
    ["pc","other","feat"],
    ["fwd","rev"],
    ["seq"],
    ["names"]
]

breakdown[0] += list(string.ascii_lowercase)

sources = None
config = None
universe = None

bp = Blueprint('browser_image',__name__)

def browser_setup(yaml_path,data_path,assets_path):
    global sources
    global config
    global universe
    
    universe = Universe(data_path)
    
    config_path = os.path.join(yaml_path,"config.yaml")
    variant_pattern = "homo_sapiens_incl_consequences-chr{0}.{1}.sorted.bed.bb"
    gc_file = os.path.join(data_path,"e2020-vcf/gc.all.bw")
    refget_hashes = os.path.join(data_path,"e2020_march_datafiles/common_files/grch38.chrom.hashes")
    variant_files = os.path.join(data_path,"e2020-vcf/bigbeds")
    config = BAIConfig(config_path,assets_path)
    seqcache = SequenceCache(refget_hashes)
    sources = BAISources(variant_files,variant_pattern,gc_file,seqcache)
    print("building locales")
    for chrom in universe.all_chroms():
        sources.add_locales(chrom,universe.locale)
    print("done")
    debug_endpoint(bp,os.path.join(yaml_path,"debug_mode.yaml"))
    return bp

class CatalogueCode(object):
    def __init__(self,wire,stick,pane,focus):
        self.wire = wire
        self.stick = stick
        self.pane = pane
        self.focus = focus

    def make_summary(self, focus_specific=False):
        return [self.wire,self.stick,self.pane,self.focus,focus_specific]

pattern = re.compile(r'[A-Z]-?[0-9]+')
def make_catalogue_codes(spec):
    for supersection in spec.split('+'):
        parts = supersection.rsplit(':',1)
        island = parts[0].rsplit('~',1)
        if len(island) == 1:
            island = (None,island[0])
        (focus,stick) = island
        for section in parts[1].split(';'):
            (tracks,leafs) = section.split('=')
            tracks = tracks.split(',')
            leafs = [x.group(0) for x in pattern.finditer(leafs)]
            for track in tracks:
                for leaf in leafs:
                    yield CatalogueCode(track,stick,leaf,focus)

class DeliveryNote(object):
    def __init__(self,catcode,got_leaf,focus_specific):
        self.code = copy.deepcopy(catcode)
        self.pane = got_leaf.pane
        self.focus_specific = focus_specific

    def make_summary(self):
        return self.code.make_summary(self.focus_specific)

test_sticks = set(["text2"])

def test_data(stick,compo):
    return []

@bp.route("/browser/data/1/<spec>")
def bulk_data(spec):
    out = []
    for code in make_catalogue_codes(spec):
        if code.stick in test_sticks:
            out.append([code.stick,code.pane,code.wire,test_data(code.stick,code.wire)])
        else:
            compo = config.tracks[code.wire]
            chrom = universe.get_from_stick(code.stick)
            leaf = Leaf(universe,code.stick,code.pane)
            (endpoint,bytecode) = config.get_endpoint(chrom,compo,code.pane[0])
            start = time.time()
            parts_in = endpoint.split("-")
            parts = [""] * (len(breakdown)+1)
            for (i,flag) in enumerate(parts_in[1:]):
                for (j,b) in enumerate(breakdown):
                    if flag in b:
                        parts[j+1] = flag
            parts[0] = parts_in[0]
            (data,got_leaf) = ([],leaf)
            if parts[0] == "contignormal":
                (data,got_leaf) = sources.contig.contig_normal(chrom,leaf,parts[3]=="seq")
            elif parts[0] == "contigshimmer":
                (data,got_leaf) = sources.contig.contig_shimmer(chrom,leaf)
            elif parts[0] == "variant":
                (data,got_leaf) = sources.variant.variant(chrom,leaf,parts[1])
            elif parts[0] == 'transcript':
                (data,got_leaf) = sources.gene.transcript(chrom,leaf,parts[1],parts[2],parts[3]=='seq',parts[4]=='names')
            elif parts[0] == 'gene':
                if parts[4] == 'names' or parts[1] == 'feat':
                    (data,got_leaf) = sources.gene.gene(chrom,leaf,parts[1],parts[2],parts[4] == 'names')
                else:
                    (data,got_leaf) = sources.gene.gene_shimmer(chrom,leaf,parts[1],parts[2])
            elif parts[0] == 'gc':
                (data,got_leaf) = sources.percgc.gc(chrom,leaf)
            out.append([code.stick,code.pane,code.wire,bytecode,code.focus,data,str(got_leaf)])
    resp = jsonify(out)
    resp.cache_control.max_age = 86400
    resp.cache_control.public = True
    return resp

@bp.route("/browser/data/3/<spec>")
def bulk_data3(spec):
    out = []
    for code in make_catalogue_codes(spec):
        compo = config.tracks[code.wire]
        chrom = universe.get_from_stick(code.stick)
        leaf = Leaf(universe,code.stick,code.pane)
        (endpoint,bytecode) = config.get_endpoint(chrom,compo,code.pane[0])
        start = time.time()
        parts_in = endpoint.split("-")
        parts = [""] * (len(breakdown)+1)
        for (i,flag) in enumerate(parts_in[1:]):
            for (j,b) in enumerate(breakdown):
                if flag in b:
                    parts[j+1] = flag
        parts[0] = parts_in[0]
        (data,got_leaf) = ([],leaf)
        if parts[0] == "contignormal":
            (data,got_leaf) = sources.contig.contig_normal(chrom,leaf,parts[3]=="seq")
        elif parts[0] == "contigshimmer":
            (data,got_leaf) = sources.contig.contig_shimmer(chrom,leaf)
        elif parts[0] == "variant":
            (data,got_leaf) = sources.variant.variant(chrom,leaf,parts[1])
        elif parts[0] == 'transcript':
            (data,got_leaf) = sources.gene.transcript(chrom,leaf,parts[1],parts[2],parts[3]=='seq',parts[4]=='names')
        elif parts[0] == 'gene':
            if parts[4] == 'names' or parts[1] == 'feat':
                (data,got_leaf) = sources.gene.gene(chrom,leaf,parts[1],parts[2],parts[4] == 'names')
            else:
                (data,got_leaf) = sources.gene.gene_shimmer(chrom,leaf,parts[1],parts[2])
        elif parts[0] == 'gc':
            (data,got_leaf) = sources.percgc.gc(chrom,leaf)
        delivery_note = DeliveryNote(code,got_leaf,config.focus_specific[code.wire])
        out.append([delivery_note.make_summary(),bytecode,data,str(got_leaf)])
    resp = jsonify(out)
    resp.cache_control.max_age = 86400
    resp.cache_control.public = True
    return resp

def make_asset(value):
    filter_ = value["filter"]
    filename = value["filename"]
    if filter_ == "png":
        pngfile = png.Reader(filename=os.path.join(config.assets_path,filename))
        (w,h,data_in,_) = pngfile.asRGBA8()
        data = b""
        for row in data_in:
            data += row
        return [[w,h],base64.b64encode(data).decode("ascii")]

@bp.route("/browser/config/<version>")
def browser_config(version):
    with open(config.config_path) as f:
        data = yaml.load(f)
        data['sticks'] = universe.get_sticks()
        api = config.get_api_config(version)
        data['bytecodes'] = api.bytecodes
        data['data-url'] = api.data_url
        data['data'] = {}
        for (name,v) in list(data['assets'].items()):
            data['data'][name] = []
            for (i,v) in enumerate(make_asset(v)):
                data['data'][name].append(v)
        return jsonify(data)

@bp.route("/browser/locale/<id_>")
def browser_locale(id_):
    resp = universe.locale.get_locale(id_)
    if resp:
        (stick,start,end) = resp
        return jsonify({
            "id": id_,
            "stick": stick,
            "start": start,
            "end": end,
            "found": True,
            "payload": [
                [
                    ["ff",stick,"X0","focus"],
                    ["ff",stick,"X0","focus"],
                    "focus",
                    [[start,end]]
                ]
            ]
        })
    else:
        return jsonify({
            "id": id_,
            "found": False
        })

def add_locales(chrom,locale):
    sources.add_locales(chrom,locale)

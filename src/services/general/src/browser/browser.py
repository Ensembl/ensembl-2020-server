from flask import jsonify, Blueprint, request
import yaml, re, time, os.path, string, base64, math, tzlocal, logging
import collections, datetime
import pyBigWig, bbi, png, pytz

from seqcache import SequenceCache

from .source.source import BAISources

from .debug import debug_endpoint
from .data import get_bigbed_data, get_bigwig_data
from .model import Leaf, Sticks, BAIConfig

breakdown = [
    ["pc","other","feat"],
    ["fwd","rev"],
    ["seq"],
    ["names"]
]

breakdown[0] += list(string.ascii_lowercase)

sources = None
config = None
sticks = None

bp = Blueprint('browser_image',__name__)

def browser_setup(yaml_path,data_path,assets_path):
    global sources
    global config
    global sticks
    
    config_path = os.path.join(yaml_path,"config.yaml")
    variant_pattern = "homo_sapiens_incl_consequences-chr{0}.{1}.sorted.bed.bb"
    chrom_sizes = os.path.join(data_path,"e2020_march_datafiles/common_files/grch38.chrom.sizes")
    contig_path = os.path.join(data_path,"e2020_march_datafiles/contigs/contigs-approx.bb")
    gene_path = os.path.join(data_path,"e2020_march_datafiles/genes_and_transcripts/canonical.bb")
    gc_file = os.path.join(data_path,"e2020-vcf/gc.all.bw")
    refget_hashes = os.path.join(data_path,"e2020_march_datafiles/common_files/grch38.chrom.hashes")
    variant_files = os.path.join(data_path,"e2020-vcf/bigbeds")
    config = BAIConfig(config_path,assets_path)    
    seqcache = SequenceCache(refget_hashes)
    sources = BAISources(gene_path,contig_path,variant_files,variant_pattern,gc_file,seqcache)
    sticks = Sticks(chrom_sizes)
    debug_endpoint(bp,os.path.join(yaml_path,"debug_mode.yaml"))
    return bp

pattern = re.compile(r'(-?[0-9]+)|([A-Za-z]+[A-Za-z-][A-Za-z])')
def break_up(spec):
    for stick in spec.split(','):
        parts = stick.split(':')
        first = None
        for part in pattern.finditer(parts[1]):
            if part.group(2):
                first = part.group(2)
            elif first:
                yield (first[:-1],parts[0],first[-1]+part.group(1))

test_sticks = set(["text2"])

def test_data(stick,compo):
    return []

@bp.route("/browser/data/1/<spec>")
def bulk_data(spec):
    out = []
    for (compo_in,stick,pane) in break_up(spec):
        if stick in test_sticks:
            out.append([stick,pane,compo_in,test_data(stick,compo_in)])
        else:
            compo = config.tracks[compo_in]
            leaf = Leaf(sticks,stick,pane)
            endpoint = config.endpoints.get((compo,pane[0]),"")
            bytecode = config.bytecodes.get((compo,pane[0]),"")
            start = time.time()
            parts_in = endpoint.split("-")
            parts = [""] * (len(breakdown)+1)
            for (i,flag) in enumerate(parts_in[1:]):
                for (j,b) in enumerate(breakdown):
                    if flag in b:
                        parts[j+1] = flag
            parts[0] = parts_in[0]
            data = []
            if parts[0] == "contignormal":
                data = sources.contig.contig_normal(leaf,parts[3]=="seq")
            elif parts[0] == "contigshimmer":
                data = sources.contig.contig_shimmer(leaf)
            elif parts[0] == "variant":
                data = sources.variant.variant(leaf,parts[1])
            elif parts[0] == 'transcript':
                data = sources.gene.transcript(leaf,parts[1],parts[2],parts[3]=='seq',parts[4]=='names')
            elif parts[0] == 'gene':
                data = sources.gene.gene(leaf,parts[1],parts[2],parts[4]=='names')
            elif parts[0] == 'gc':
                data = sources.percgc.gc(leaf)
            out.append([stick,pane,compo_in,bytecode,data])
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

@bp.route("/browser/config")
def browser_config():
    with open(config.config_path) as f:
        data = yaml.load(f)
        data['sticks'] = sticks.get_sticks()
        data['data'] = {}
        for (name,v) in list(data['assets'].items()):
            data['data'][name] = []
            for (i,v) in enumerate(make_asset(v)):
                data['data'][name].append(v)
        return jsonify(data)

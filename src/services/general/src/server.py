#! /usr/bin/env python3

from flask import Flask, jsonify, request
from flask_cors import CORS
import os.path, string, time, yaml, re, base64
import pyBigWig, bbi, png
import shimmer
from seqcache import SequenceCache
import urllib, urllib.parse, math

app = Flask(__name__)
CORS(app)

home_dir = "/home/dan"
#home_dir = "/Users/sboddu/e2020"
data_repo = home_dir + "/e2020_march_datafiles"

refget_hashes = data_repo + "/common_files/grch38.chrom.hashes"
config_path = home_dir + "/ensembl-2020-server/src/services/general/src/yaml/config.yaml"
contig_path = data_repo + "/contigs/contigs-approx.bb"
gene_path = data_repo + "/genes_and_transcripts/canonical.bb"
chrom_sizes= data_repo + "/common_files/grch38.chrom.sizes"
variant_files = home_dir + "/e2020-vcf/bigbeds"
objects_list_path = home_dir + "/ensembl-2020-server/src/services/general/src/yaml/example_objects.yaml"
objects_info_path = home_dir + "/ensembl-2020-server/src/services/general/src/yaml/objects_info.yaml"
gc_file = home_dir + "/e2020-vcf/gc.all.bw"
assets_path = home_dir + "/ensembl-2020-server/src/services/general/src/assets"

variant_pattern = "homo_sapiens_incl_consequences-chr{0}.{1}.sorted.bed.bb"

T = "track:"
ep_map = {}
bc_map = {}

endpoints = {}
bytecodes = {}
tracks = {}
with open(config_path) as f:
    bc = yaml.load(f)
    for (ep_name,v) in bc["endpoints"].items():
        if "endpoint" in v:
            ep_map[ep_name] = v["endpoint"]
        if "bytecode" in v:
            bc_map[ep_name] = v["bytecode"]
    for (track_name,v) in bc["tracks"].items():
        for (code,v) in v["endpoints"].items():
            for scale in range(ord(code[0]),ord(code[1])+1):
                if v["endpoint"] in ep_map:
                    endpoints[(track_name,chr(scale))] = ep_map[v["endpoint"]]
                if v["endpoint"] in bc_map:
                    bytecodes[(track_name,chr(scale))] = bc_map[v["endpoint"]]
    for (t_name,v) in bc["tracks"].items():
        if "wire" in v:
            tracks[v["wire"]] = t_name

seqcache = SequenceCache(refget_hashes)

def get_sticks():
    out = {}
    with open(chrom_sizes) as f:
        for line in f.readlines():
            (f_chr,f_len) = line.strip().split("\t")
            out[f_chr] = f_len
    out["text2"] = "1000000"
    return out

def bounds_fix(chrom,start,end):
    sticks = get_sticks()
    if chrom in sticks:
        f_len = sticks[chrom]
        f_len = int(f_len)
        if end >= f_len:
            end = int(f_len)-1
        if start >= f_len:
            start = int(f_len)-2                    
    if start < 0:
        start = 0
    return (start,end)

def get_bigbed_data(path,chromosome,start,end):
    bb = pyBigWig.open(path)
    try:
        out = bb.entries(chromosome,start,end) or []
    except (RuntimeError,OverflowError):
        out = []
    bb.close()
    return out

def get_bigwig_data(path,chrom,start,end,points):
    if os.path.exists(path):
        try:
            return bbi.fetch(path,chrom,start,end,bins=points)
        except (KeyError,OverflowError):
            pass
    return []

def burst_leaf(leaf):
    (chrom,rest) = leaf.rsplit(':',1)
    (start,end) = rest.split('-',1)
    try:
        (start,end) = (int(start),int(end))
    except ValueError:
        return (chrom,1,2)
    (start,end) = bounds_fix(chrom,start,end)
    return (chrom,start,end)


def make_asset(value):
    filter_ = value["filter"]
    filename = value["filename"]
    if filter_ == "png":
        pngfile = png.Reader(filename=os.path.join(assets_path,filename))
        (w,h,data_in,_) = pngfile.asRGBA8()
        data = b""
        for row in data_in:
            data += row
        return [[w,h],base64.b64encode(data).decode("ascii")]

@app.route("/browser/config")
def browser_config():
    with open(config_path) as f:
        data = yaml.load(f)
        data['sticks'] = get_sticks()
        data['data'] = {}
        for (name,v) in list(data['assets'].items()):
            data['data'][name] = []
            for (i,v) in enumerate(make_asset(v)):
                data['data'][name].append(v)
        return jsonify(data)

FEATURED=set(["BRCA2","TTN"])

def gene_gene(leaf,type_,dir_,get_names):
    (chrom,leaf_start,leaf_end) = burst_leaf(leaf)
    data = get_bigbed_data(gene_path,chrom,leaf_start,leaf_end)
    out_starts = []
    out_lens = []
    names = ""
    name_lens = []
    colour = 1 if type_ == 'pc' else 0
    for line in data:
        gene_start = int(line[0])
        gene_end = int(line[1])
        parts = line[2].split("\t")
        (biotype,gene_name,strand) = (parts[16],parts[15],parts[2])
        if type_ == 'feat':
            colour = 2
            if gene_name not in FEATURED:
                continue
            dir_ = ("fwd" if strand == '+' else "rev")
        else:
            if gene_name in FEATURED:
                continue
            if (strand == '+') != (dir_ == 'fwd'):
                continue
            if (biotype == 'protein_coding') != (type_ == 'pc'):
                continue
        out_starts.append(gene_start)
        out_lens.append(gene_end-gene_start)
        if get_names:
            name_lens.append(len(gene_name))
            names += gene_name
    if dir_ == 'fwd':
        dir_ = 1
    elif dir_ == 'rev':
        dir_ = 0
    else:
        dir_ = 2
    return [out_starts,out_lens,names,name_lens,[colour,dir_]]

MIN_WIDTH = 1000

def gene_transcript(leaf,type_,dir_,seq,names,scale):
    min_bp = calc_bp_px(scale) / MIN_WIDTH
    (chrom,leaf_start,leaf_end) = burst_leaf(leaf)
    data = get_bigbed_data(gene_path,chrom,leaf_start,leaf_end)
    out_starts = []
    out_lens = []
    out_nump = []
    out_pattern = []
    out_utrs = []
    out_exons = []
    out_introns = []
    seq_req = []
    names = ""
    name_lens = []
    colour = 1 if type_ == 'pc' else 0
    for line in data:
        gene_start = int(line[0])
        gene_end = int(line[1])
        parts = line[2].split("\t")
        (
            biotype,gene_name,part_starts,part_lens,cds_start,cds_end,
            strand
        ) = (
            parts[16],parts[15],parts[8],parts[7],parts[3],parts[4],
            parts[2]
        )
        if type_ == 'feat':
            colour = 2
            if gene_name not in FEATURED:
                continue
            dir_ = ("fwd" if strand == '+' else "rev")
        else:
            if gene_name in FEATURED:
                continue
            if (strand == '+') != (dir_ == 'fwd'):
                continue
            if (biotype == 'protein_coding') != (type_ == 'pc'):
                continue
        seq_req.append((max(gene_start,leaf_start),min(gene_end,leaf_end)))
        name_lens.append(len(gene_name))
        names += gene_name
        if part_starts.endswith(","): part_starts = part_starts[:-1]
        if part_lens.endswith(","): part_lens = part_lens[:-1]
        part_starts = [int(x) for x in part_starts.split(",")]
        part_lens = [int(x) for x in part_lens.split(",")]
        cds_start = int(cds_start) - gene_start
        cds_end = int(cds_end) - gene_start
        # build basic intron/exon pattern (split, but don't mark UTR)
        blocks = []
        prev_exon_end = 0
        undershoot = 0
        for (exon_start,exon_len) in zip(part_starts,part_lens):
            new_undershoot = max(min_bp-exon_len,0)
            # intron between previous exan and this one
            if exon_start != prev_exon_end:
                intron_start = prev_exon_end
                intron_len = exon_start - prev_exon_end
                if undershoot > 0:
                    stolen = min(undershoot,intron_len)
                    blocks[-1][2] += stolen
                    intron_len -= stolen
                blocks.append([2,intron_start,intron_len])
            # if 5' is in this exon, split that off now
            if cds_start > exon_start and cds_start < exon_start+exon_len:
                blocks.append([1,exon_start,cds_start-exon_start])
                exon_len -= cds_start-exon_start
                exon_start = cds_start
            # if 3' is in this exon, split of main body now
            if cds_end > exon_start and cds_end < exon_start+exon_len:
                blocks.append([1,exon_start,cds_end-exon_start])
                exon_len -= cds_end-exon_start
                exon_start = cds_end
            # whatever remains of this exon (main or 3')
            blocks.append([1,exon_start,exon_len])
            prev_exon_end = exon_start + exon_len
            undershoot = new_undershoot
        # mark UTRs
        for b in blocks:
            if b[0] == 1 and (b[1] < cds_start or b[1] >= cds_end):
                b[0] = 0
        # put into output strucutre
        out_starts.append(gene_start)
        out_lens.append(gene_end-gene_start)
        out_nump.append(len(blocks))
        for b in blocks:
            out_pattern.append(b[0])
            if b[0] == 2:
                out_introns.append(b[2])
            elif b[0] == 1:
                out_exons.append(b[2])
            else:
                out_utrs.append(b[2])
    if dir_ == 'fwd':
        dir_ = 1
    elif dir_ == 'rev':
        dir_ = 0
    else:
        dir_ = 2
    data = [out_starts,out_nump,out_pattern,out_utrs,out_exons,
            out_introns,names,name_lens,[colour,dir_],out_lens]
    if seq:
        (seq_text,seq_starts,seq_lens) = seqcache.get(chrom,seq_req)
        data += [seq_text,seq_starts,seq_lens]
    return data

def contig_shimmer(leaf):
    return contig_full(leaf,True,False)

def contig_normal(leaf,seq):
    return contig_full(leaf,False,seq)

def contig_full(leaf,do_shimmer,seq):
    starts = []
    lens = []
    senses = []
    (chrom,leaf_start,leaf_end) = burst_leaf(leaf)
    data = get_bigbed_data(contig_path,chrom,leaf_start,leaf_end)
    for (start,end,extra) in data:
        extra = extra.split("\t")
        starts.append(start)
        lens.append(end-start)
        senses.append(extra[2]=='+')
    if do_shimmer:
        (starts, lens, senses) = shimmer.shimmer(starts,lens,senses,leaf_start,leaf_end)
    data = []
    if seq:
        (seq_text,seq_starts,seq_lens) = seqcache.get(chrom,[(leaf_start,leaf_end)])
        data += [seq_text,seq_starts,seq_lens]
    elif leaf_end - leaf_start < 40000:
        # prime cache
        seqcache.get(chrom,[(leaf_start,leaf_end)])
    data += [starts,lens,senses]
    return data

@app.route("/browser/example_objects")
def example_objects():
    with open(objects_list_path) as f:
        data = yaml.load(f)
        return jsonify(data)


@app.route("/browser/get_object_info/<object_id>")
def get_object_info(object_id):
    with open(objects_info_path) as f:
        data = yaml.load(f)
        if object_id not in data:
          return jsonify({'error':'Object Not Found'})
        else:
          return jsonify(data[object_id])


var_category = {
    '3_prime_UTR_variant': 2,
    '5_prime_UTR_variant': 2,
    'coding_sequence_variant': 3,
    'downstream_gene_variant': 2,
    'feature_elongation': 1,
    'feature_truncation': 1,
    'frameshift_variant': 5,
    'incomplete_terminal_codon_variant': 3,
    'inframe_deletion': 4,
    'inframe_insertion': 4,
    'intergenic_variant': 1,
    'intron_variant': 2,
    'mature_miRNA_variant': 3,
    'missense_variant': 4,
    'NMD_transcript_variant': 2,
    'non_coding_transcript_exon_variant': 2,
    'non_coding_transcript_variant': 2,
    'protein_altering_variant': 4,
    'regulatory_region_ablation': 4,
    'regulatory_region_amplification': 1,
    'regulatory_region_fusion': 1,
    'regulatory_region_translocation': 1,
    'regulatory_region_variant': 1,
    'splice_acceptor_variant': 5,
    'splice_donor_variant': 5,
    'splice_region_variant': 3,
    'start_lost': 5,
    'start_retained_variant': 3,
    'stop_retained_variant': 3,
    'stop_gained': 5,
    'stop_lost': 5,
    'synonymous_variant': 3,
    'TFBS_ablation': 1,
    'TFBS_amplification': 1,
    'TFBS_fusion': 1,
    'TFBS_translocation': 1,
    'TF_binding_site_variant': 1,
    'transcript_ablation': 5,
    'transcript_amplification': 5,
    'transcript_fusion': 4,
    'transcript_translocation': 2,
    'upstream_gene_variant': 2,
}

def variant(leaf,scale):
    starts = []
    lens = []
    types = []
    (chrom,leaf_start,leaf_end) = burst_leaf(leaf)
    path = os.path.join(variant_files,variant_pattern.format(chrom,scale))
    if os.path.exists(path):
        data = get_bigbed_data(path,chrom,leaf_start,leaf_end)
        for (start,end,extra) in data:
            vc = var_category.get(extra,0)
            if len(starts) and starts[-1] == start:
                types[-1] = max(vc,types[-1])
            else:
                starts.append(start)
                lens.append(end-start)
                types.append(vc)
    else:
        print('missing',path)
    return [starts,lens,types]

POINTS = 40
def gc(leaf):
    (chrom,leaf_start,leaf_end) = burst_leaf(leaf)
    steps = 500
    y = get_bigwig_data(gc_file,chrom,leaf_start,leaf_end,steps)
    y = [ int((y or 0)*POINTS/100) for y in y ]
    return [[leaf_start,leaf_end],y,[0.5],[1/POINTS]]

def extract_bulk_parts():
    out = []
    # allow parts qp for debugging, etc
    for part in request.args.getlist('parts'):
        out.append(part.split("/",2))
    return out

def calc_bp_px(spec):
    spec_number = ord(spec) - ord('A') - 13
    bp_px = 10**(math.floor(abs(spec_number)/2))
    if abs(spec_number) % 2:
        bp_px *= 3
    if spec_number > 0:
        bp_px = 1.0 / bp_px
    return bp_px * 5000

def leaf_range(chrom,spec):
    bp_px = calc_bp_px(spec[0])
    pos = int(spec[1:])
    return "{0}:{1}-{2}".format(chrom,
    math.floor(pos*bp_px),
    math.ceil((pos+1)*bp_px))

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

def test_data(stick,compo):
    return []

breakdown = [
    ["pc","other","feat"],
    ["fwd","rev"],
    ["seq"],
    ["names"]
]

breakdown[0] += list(string.ascii_lowercase)

test_sticks = set(["text2"])

@app.route("/browser/data/1/<spec>")
def bulk_data(spec):
    out = []
    for (compo_in,stick,pane) in break_up(spec):
        if stick in test_sticks:
            out.append([stick,pane,compo_in,test_data(stick,compo_in)])
        else:
            compo = tracks[compo_in]
            leaf = leaf_range(stick,pane)
            endpoint = endpoints.get((compo,pane[0]),"")
            bytecode = bytecodes.get((compo,pane[0]),"")
            print("{0} -> {1}".format(endpoint,bytecode))
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
                data = contig_normal(leaf,parts[3]=="seq")
            elif parts[0] == "contigshimmer":
                data = contig_shimmer(leaf)
            elif parts[0] == "variant":
                data = variant(leaf,parts[1])
            elif parts[0] == 'transcript':
                data = gene_transcript(leaf,parts[1],parts[2],parts[3]=='seq',parts[4]=='names',pane[0])
            elif parts[0] == 'gene':
                data = gene_gene(leaf,parts[1],parts[2],parts[4]=='names')
            elif parts[0] == 'gc':
                data = gc(leaf)
            out.append([stick,pane,compo_in,bytecode,data])
    resp = jsonify(out)
    resp.cache_control.max_age = 86400
    resp.cache_control.public = True
    return resp

    return resp
    
if __name__ == "__main__":
   app.run(port=4000)

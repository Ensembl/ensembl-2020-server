#! /usr/bin/env python3

from flask import Flask, jsonify, request
from flask_cors import CORS
import yaml
import pyBigWig
import shimmer
import urllib, urllib.parse

app = Flask(__name__)
CORS(app)

home_dir = "/home/dan"
#home_dir = "/Users/sboddu/e2020"
data_repo = home_dir + "/e2020_march_datafiles"

refget_hashes = data_repo + "/common_files/grch38.chrom.hashes"
config_path = home_dir + "/ensembl-server/demo/flask/yaml/config.yaml"
contig_path = data_repo + "/contigs/contigs-approx.bb"
gene_path = data_repo + "/genes_and_transcripts/canonical.bb"
chrom_sizes= data_repo + "/common_files/grch38.chrom.sizes"
variant_z = home_dir + "/tmp/chr6-z.bb"
objects_list_path = home_dir + "/ensembl-server/demo/flask/yaml/example_objects.yaml"
objects_info_path = home_dir + "/ensembl-server/demo/flask/yaml/objects_info.yaml"

def get_sticks():
    out = {}
    with open(chrom_sizes) as f:
        for line in f.readlines():
            (f_chr,f_len) = line.strip().split("\t")
            out[f_chr] = f_len
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
    out = bb.entries(chromosome,start,end) or []
    bb.close()
    return out

def burst_leaf(leaf):
    (chrom,rest) = leaf.rsplit(':',1)
    (start,end) = rest.split('-',1)
    try:
        (start,end) = (int(start),int(end))
    except ValueError:
        return (chrom,1,2)
    (start,end) = bounds_fix(chrom,start,end)
    return (chrom,start,end)


@app.route("/browser/config")
def browser_config():
    with open(config_path) as f:
        data = yaml.load(f)
        data['sticks'] = get_sticks()
        return jsonify(data)


@app.route("/browser/data/gene/<leaf>")
def gene_gene(leaf):
    (chrom,leaf_start,leaf_end) = burst_leaf(leaf)
    data = get_bigbed_data(gene_path,chrom,leaf_start,leaf_end)
    out_starts = []
    out_lens = []
    for line in data:
        gene_start = int(line[0])
        gene_end = int(line[1])
        parts = line[2].split("\t")
        (biotype,gene_name,strand) = (parts[16],parts[15],parts[2])
        dir_ = request.args.get('dir')
        type_ = request.args.get('type')
        if (strand == '+') != (dir_ == 'fwd'):
            continue
        if (biotype == 'protein_coding') != (type_ == 'pc'):
            continue
        out_starts.append(gene_start)
        out_lens.append(gene_end-gene_start)
    data = {'data':[out_starts,out_lens]}
    return jsonify(data)

def refget(hash_,start,end):
    url = ("https://www.ebi.ac.uk/ena/cram/sequence/{}?start={}&end={}"
            .format(hash_,start,end))
    headers = {'Accept': 'text/vnd.ga4gh.refget.v1.0.0+plain;charset=us-ascii'}
    req = urllib.request.Request(url, None, headers)    
    with urllib.request.urlopen(req) as response:
        html = response.read()
        return html.decode("ascii")

def get_sequence(chrom,requests):
    seq_text = ""
    seq_starts = []
    seq_lens = []
    hash_ = None
    with open(refget_hashes) as f:
        for line in f.readlines():
            parts = line.split("\t")
            if chrom == parts[0]:
                hash_ = parts[1]
    if hash_:
        for (start,end) in requests:
            seq = refget(hash_,start,end)
            seq_starts.append(start)
            seq_lens.append(len(seq))
            seq_text += seq
    return (seq_text,seq_starts,seq_lens)

@app.route("/browser/data/transcript/<leaf>")
def gene_transcript(leaf):
    (chrom,leaf_start,leaf_end) = burst_leaf(leaf)
    data = get_bigbed_data(gene_path,chrom,leaf_start,leaf_end)
    out_starts = []
    out_nump = []
    out_pattern = []
    out_utrs = []
    out_exons = []
    out_introns = []
    seq_req = []
    for line in data:
        gene_start = int(line[0])
        gene_end = int(line[1])
        seq_req.append((max(gene_start,leaf_start),min(gene_end,leaf_end)))
        parts = line[2].split("\t")
        (
            biotype,gene_name,part_starts,part_lens,cds_start,cds_end,
            strand
        ) = (
            parts[16],parts[15],parts[8],parts[7],parts[3],parts[4],
            parts[2]
        )
        dir_ = request.args.get('dir')
        type_ = request.args.get('type')
        if (strand == '+') != (dir_ == 'fwd'):
            continue
        if (biotype == 'protein_coding') != (type_ == 'pc'):
            continue
        if part_starts.endswith(","): part_starts = part_starts[:-1]
        if part_lens.endswith(","): part_lens = part_lens[:-1]
        part_starts = [int(x) for x in part_starts.split(",")]
        part_lens = [int(x) for x in part_lens.split(",")]
        cds_start = int(cds_start) - gene_start
        cds_end = int(cds_end) - gene_start
        # build basic intron/exon pattern (split, but don't mark UTR)
        blocks = []
        prev_exon_end = 0
        for (exon_start,exon_len) in zip(part_starts,part_lens):
            if exon_start != prev_exon_end:
                blocks.append([2,prev_exon_end,exon_start-prev_exon_end])
            if cds_start > exon_start and cds_start < exon_start+exon_len:
                blocks.append([1,exon_start,cds_start-exon_start])
                exon_len -= cds_start-exon_start
                exon_start = cds_start
            if cds_end > exon_start and cds_end < exon_start+exon_len:
                blocks.append([1,exon_start,cds_end-exon_start])
                exon_len -= cds_end-exon_start
                exon_start = cds_end
            blocks.append([1,exon_start,exon_len])
            prev_exon_end = exon_start + exon_len
        # mark UTRs
        for b in blocks:
            if b[0] == 1 and (b[1] < cds_start or b[1] >= cds_end):
                b[0] = 0
        # put into output strucutre
        out_starts.append(gene_start)
        out_nump.append(len(blocks))
        for b in blocks:
            out_pattern.append(b[0])
            if b[0] == 2:
                out_introns.append(b[2])
            elif b[0] == 1:
                out_exons.append(b[2])
            else:
                out_utrs.append(b[2])
    data = [out_starts,out_nump,out_pattern,out_utrs,out_exons,out_introns]
    if request.args.get('seq') == 'yes':
        (seq_text,seq_starts,seq_lens) = get_sequence(chrom,seq_req)
        data += [seq_text,seq_starts,seq_lens]
    data = {'data': data}
    return jsonify(data)

@app.route("/browser/data/contig-shimmer/<leaf>")
def contig_shimmer(leaf):
    return contig_full(leaf,True)

@app.route("/browser/data/contig-normal/<leaf>")
def contig_normal(leaf):
    return contig_full(leaf,False)

def contig_full(leaf,do_shimmer):
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
    if request.args.get('seq') == 'yes':
        (seq_text,seq_starts,seq_lens) = get_sequence(chrom,[(leaf_start,leaf_end)])
        data += [seq_text,seq_starts,seq_lens]
    data += [starts,lens,senses]
    data = {'data': data }
    return jsonify(data)

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

@app.route("/browser/data/variant/<leaf>")
def variant(leaf):
    starts = []
    lens = []
    types = []
    if False:
        (chrom,leaf_start,leaf_end) = burst_leaf(leaf)
        data = get_bigbed_data(variant_z,chrom,leaf_start,leaf_end)
        for (start,end,extra) in data:
            starts.append(start)
            lens.append(end-start)
            types.append(var_category.get(extra,0))
            if extra not in var_category:
                print('missing',extra)
    return jsonify({'data': [starts,lens,types]})

if __name__ == "__main__":
   app.run(port=4000)

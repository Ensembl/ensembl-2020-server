#! /usr/bin/env python3

from flask import Flask, jsonify, request
from flask_cors import CORS
import yaml
import pyBigWig
import shimmer

app = Flask(__name__)
CORS(app)

config_path = "/home/dan/ensembl-server/demo/flask/config.yaml"
contig_path = "/home/dan/e2020_march_datafiles/contigs/contigs-approx.bb"
gene_path = "/home/dan/e2020_march_datafiles/genes_and_transcripts/canonical.bb"
chrom_sizes= "/home/dan/e2020_march_datafiles/common_files/grch38.chrom.sizes"
objects_list_path = "/Users/sboddu/e2020/ensembl-server/demo/flask/example_objects.yaml"

def bounds_fix(chrom,start,end):
    with open(chrom_sizes) as f:
        for line in f.readlines():
            (f_chr,f_len) = line.strip().split("\t")
            if f_chr == chrom:
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
    chrom = "6" # XXX
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
        return jsonify(data)

@app.route("/browser/data/transcript/<leaf>")
def gene_gene(leaf):
    (chrom,leaf_start,leaf_end) = burst_leaf(leaf)
    data = get_bigbed_data(gene_path,chrom,leaf_start,leaf_end)
    out_starts = []
    out_nump = []
    out_pattern = []
    out_utrs = []
    out_exons = []
    out_introns = []
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
    data = {'data':[out_starts,out_nump,out_pattern,out_utrs,out_exons,out_introns]}
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
    data = {'data':[starts,lens,senses]}
    return jsonify(data)

@app.route("/browser/example_objects")
def example_objects():
    with open(objects_list_path) as f:
        data = yaml.load(f)
        return jsonify(data)


 
if __name__ == "__main__":
   app.run(port=4000)

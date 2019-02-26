#! /usr/bin/env python3

from flask import Flask, jsonify
from flask_cors import CORS
import yaml
import pyBigWig
import shimmer

app = Flask(__name__)
CORS(app)

config_path = "/home/dan/ensembl-server/demo/flask/config.yaml"
contig_path = "/home/dan/e2020_march_datafiles/contigs/contigs-approx.bb"
chrom_sizes= "/home/dan/e2020_march_datafiles/common_files/grch38.chrom.sizes"

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

def get_contig_data(chromosome,start,end):
    bb = pyBigWig.open(contig_path)
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
    data = get_contig_data(chrom,leaf_start,leaf_end)
    for (start,end,extra) in data:
        extra = extra.split("\t")
        starts.append(start)
        lens.append(end-start)
        senses.append(extra[2]=='+')
    if do_shimmer:
        (starts, lens, senses) = shimmer.shimmer(starts,lens,senses,leaf_start,leaf_end)
    data = {'data':[starts,lens,senses]}
    return jsonify(data)
  
if __name__ == "__main__":
   app.run(port=4000)

#! /usr/bin/env python3

from flask import Flask, jsonify
from flask_cors import CORS
import yaml

app = Flask(__name__)
CORS(app)

import pyBigWig

config_path = "/home/dan/ensembl-server/demo/flask/config.yaml"
contig_path = "/home/dan/e2020_march_datafiles/contigs/contigs-approx.bb"

def get_contig_data(chromosome,start,end):
    bb = pyBigWig.open(contig_path)
    out = bb.entries(chromosome,start,end) or []
    bb.close()
    return out

def burst_leaf(leaf):
    (chrom,rest) = leaf.rsplit(':',1)
    (start,end) = rest.split('-',1)
    chrom = "6" # XXX
    return (chrom,int(start),int(end))

@app.route("/browser/config")
def browser_config():
    with open(config_path) as f:
        data = yaml.load(f)
        return jsonify(data)

@app.route("/browser/data/contig-normal/<leaf>")
def contig_normal(leaf):
    starts = []
    lens = []
    senses = []
    try:
        (chrom,start,end) = burst_leaf(leaf)
        data = get_contig_data(chrom,start,end)
        for (start,end,extra) in data:
            extra = extra.split("\t")
            starts.append(start)
            lens.append(end-start)
            senses.append(extra[2]=='+')
    except ValueError:
        pass
    data = {'data':[starts,lens,senses]}
    return jsonify(data)
  
if __name__ == "__main__":
   app.run(port=4000)

#! /usr/bin/env python3

from flask import Flask, jsonify

app = Flask(__name__)

import pyBigWig

contig_path = "/home/dan/e2020_march_datafiles/contigs/contigs-approx.bb"

def get_contig_data(chromosome,start,end):
    bb = pyBigWig.open(contig_path)
    out = bb.entries(chromosome,start,end) or []
    print(out)
    bb.close()
    return out

def burst_leaf(leaf):
    (chrom,rest) = leaf.rsplit(':',2)
    (start,end) = rest.split('-')
    return (chrom,int(start),int(end))

@app.route("/browser/data/contig-normal/<leaf>")
def contig_normal(leaf):
    (chrom,start,end) = burst_leaf(leaf)
    data = get_contig_data(chrom,start,end)
    starts = []
    lens = []
    for (start,end,_) in data:
        starts.append(start)
        lens.append(end-start)
    data = {'data':[starts,lens]}
    return jsonify(data)
  
if __name__ == "__main__":
   app.run(port=4000)

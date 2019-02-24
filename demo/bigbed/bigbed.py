#! /usr/bin/env python3

# https://packages.debian.org/sid/science/python3-pybigwig
# pip install -e ~/path

import pyBigWig

contig_path = "/home/dan/e2020_march_datafiles/contigs/contigs-approx.bb"

def get_data(chromosome,start,end):
    bb = pyBigWig.open(contig_path)
    print(bb.entries(chromosome,start,end))
    bb.close()

get_data('1',10000000,10002000)

import os.path
import pyBigWig, bbi

def get_bigbed_data(path,chromosome,start,end):
    bb = pyBigWig.open(path)
    try:
        out = bb.entries(chromosome,start,end) or []
    except (RuntimeError,OverflowError):
        out = []
    bb.close()
    return out

def get_chrom_length(path,chrom):
    bb = pyBigWig.open(path)
    try:
        return bb.chroms(chrom) or None
    except (RuntimeError,OverflowError):
        return []
    finally:
        bb.close()

def get_bigwig_data(path,chrom,start,end,points):
    if os.path.exists(path):
        try:
            return bbi.fetch(path,chrom,start,end,bins=points)
        except (KeyError,OverflowError):
            pass
    return []

import os.path
from ..data import get_bigwig_data

POINTS = 40

class BAISPercGC(object):
    def __init__(self,gc_file):
        self.gc_file = gc_file
    
    def gc(self,chrom,leaf):
        steps = 500
        path = chrom.file_path("gc","gc.bw")
        chrom_path = chrom.file_path("gc","{0}.bw".format(chrom.seq_hash))
        if os.path.exists(path):
            y = get_bigwig_data(path,chrom.name,leaf.start,leaf.end,steps)
        elif os.path.exists(chrom_path):
            y = get_bigwig_data(chrom_path,chrom.name,leaf.start,leaf.end,steps)
        y = [ int((y or 0)*POINTS/100) for y in y ]
        return ([[leaf.start,leaf.end],y,[0.5],[1/POINTS]],leaf)

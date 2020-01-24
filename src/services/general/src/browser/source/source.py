from .contig import BAISContig
from .gc import BAISPercGC
from .gene import BAISGeneTranscript
from .variant import BAISVariant

class BAISources(object):
    def __init__(self,gc_file,seqcache):
       self.contig = BAISContig(seqcache) 
       self.percgc = BAISPercGC(gc_file)
       self.variant = BAISVariant()
       self.gene = BAISGeneTranscript(seqcache)

    def add_locales(self,chrom,locales):
        self.gene.add_locales(chrom,locales)

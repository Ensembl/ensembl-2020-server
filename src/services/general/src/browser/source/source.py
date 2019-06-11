from .contig import BAISContig
from .gc import BAISPercGC
from .gene import BAISGeneTranscript
from .variant import BAISVariant

class BAISources(object):
    def __init__(self,gene_path,contig_path,variant_files,variant_pattern,gc_file,seqcache):
       self.contig = BAISContig(contig_path,seqcache) 
       self.percgc = BAISPercGC(gc_file)
       self.variant = BAISVariant(variant_files,variant_pattern)
       self.gene = BAISGeneTranscript(gene_path,seqcache)


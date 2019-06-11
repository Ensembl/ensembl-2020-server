from ..shimmer import shimmer
from ..data import get_bigbed_data

class BAISContig(object):
    def __init__(self,contig_path,seqcache):
        self.contig_path = contig_path
        self.seqcache = seqcache

    def contig_shimmer(self,leaf):
        return self._contig_full(leaf,True,False)

    def contig_normal(self,leaf,seq):
        return self._contig_full(leaf,False,seq)

    def _contig_full(self,leaf,do_shimmer,seq):
        starts = []
        lens = []
        senses = []
        data = get_bigbed_data(self.contig_path,leaf.chrom,leaf.start,leaf.end)
        for (start,end,extra) in data:
            extra = extra.split("\t")
            starts.append(start)
            lens.append(end-start)
            senses.append(extra[2]=='+')
        if do_shimmer:
            (starts, lens, senses) = shimmer(starts,lens,senses,leaf.start,leaf.end)
        data = []
        if seq:
            (seq_text,seq_starts,seq_lens) = self.seqcache.get(leaf.chrom,[(leaf.start,leaf.end)])
            data += [seq_text,seq_starts,seq_lens]
        elif leaf.end - leaf.start < 40000:
            # prime cache
            self.seqcache.get(leaf.chrom,[(leaf.start,leaf.end)])
        data += [starts,lens,senses]
        return data


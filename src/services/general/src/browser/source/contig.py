from ..shimmer import shimmer
from ..data import get_bigbed_data

class BAISContig(object):
    def __init__(self,seqcache):
        self.seqcache = seqcache

    def contig_shimmer(self,chrom,leaf):
        return self._contig_full(chrom,leaf,True,False)

    def contig_normal(self,chrom,leaf,seq):
        return self._contig_full(chrom,leaf,False,seq)

    def _contig_full(self,chrom,leaf,do_shimmer,seq):
        starts = []
        lens = []
        senses = []
        path = chrom.file_path("contigs","contigs.bb")
        try:
            data = get_bigbed_data(path,chrom.name,leaf.start,leaf.end)
            for (start,end,extra) in data:
                extra = extra.split("\t")
                start = max(start,leaf.start)
                end = min(end,leaf.end)
                if end > start:
                    starts.append(start)
                    lens.append(end-start)
                    senses.append(extra[2]=='+')
        except RuntimeError:
            starts.append(leaf.start)
            lens.append(leaf.end-leaf.start+1)
            senses.append(True)
        if do_shimmer:
            (starts, lens, senses) = shimmer(starts,lens,senses,leaf.start,leaf.end)
        data = []
        if seq:
            (seq_text,seq_starts) = self.seqcache.get(chrom,[(leaf.start,leaf.end)])
            data += [{ "string": seq_text },seq_starts]
        elif leaf.end - leaf.start < 40000:
            # prime cache
            self.seqcache.get(chrom,[(leaf.start,leaf.end)])
        data += [starts,lens,senses]
        return (data,leaf)


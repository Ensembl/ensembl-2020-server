import os.path
from ..data import get_bigbed_data

var_category = {
    '3_prime_UTR_variant': 2,
    '5_prime_UTR_variant': 2,
    'coding_sequence_variant': 3,
    'downstream_gene_variant': 2,
    'feature_elongation': 1,
    'feature_truncation': 1,
    'frameshift_variant': 5,
    'incomplete_terminal_codon_variant': 3,
    'inframe_deletion': 4,
    'inframe_insertion': 4,
    'intergenic_variant': 1,
    'intron_variant': 2,
    'mature_miRNA_variant': 3,
    'missense_variant': 4,
    'NMD_transcript_variant': 2,
    'non_coding_transcript_exon_variant': 2,
    'non_coding_transcript_variant': 2,
    'protein_altering_variant': 4,
    'regulatory_region_ablation': 4,
    'regulatory_region_amplification': 1,
    'regulatory_region_fusion': 1,
    'regulatory_region_translocation': 1,
    'regulatory_region_variant': 1,
    'splice_acceptor_variant': 5,
    'splice_donor_variant': 5,
    'splice_region_variant': 3,
    'start_lost': 5,
    'start_retained_variant': 3,
    'stop_retained_variant': 3,
    'stop_gained': 5,
    'stop_lost': 5,
    'synonymous_variant': 3,
    'TFBS_ablation': 1,
    'TFBS_amplification': 1,
    'TFBS_fusion': 1,
    'TFBS_translocation': 1,
    'TF_binding_site_variant': 1,
    'transcript_ablation': 5,
    'transcript_amplification': 5,
    'transcript_fusion': 4,
    'transcript_translocation': 2,
    'upstream_gene_variant': 2,
}

class BAISVariant(object):
    def __init__(self):
        pass

    def variant(self,chrom,leaf,scale):
        starts = []
        lens = []
        types = []
        filename = "{0}${1}.{2}.bb".format(chrom.genome_id,chrom.name,scale)
        path = chrom.file_path("genes_and_transcripts",filename)
        path = chrom.file_path("variants",filename)
        if os.path.exists(path):
            data = get_bigbed_data(path,chrom.name,leaf.start,leaf.end)
            for (start,end,extra) in data:
                vc = var_category.get(extra,0)
                if len(starts) and starts[-1] == start:
                    types[-1] = max(vc,types[-1])
                else:
                    starts.append(start)
                    lens.append(end-start)
                    types.append(vc)
        else:
            print('missing',path)
        return ([starts,lens,types],leaf)

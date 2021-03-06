import re

from ..data import get_bigbed_data, get_chrom_length
from ..shimmer import shimmer

FEATURED=set(["BRCA2","TraesCS3D02G273600","PF3D7_1143500","grpE","SFA1","sms-2"])
MIN_WIDTH = 1000

# HACK Strip versions from IDs. We have discussed but noy yet agreed on whether versions 
# should be in IDs or not. For now it's simplest to strip them (as most places they are
# already missing) until we have a decision. -- Jul2019
id_strip = re.compile(r'\.\d+$')

# HACK should use correct codes in the first place
def munge_code(s):
    s = re.sub(r'_',' ',s)
    if s == 'mane select':
        s = "MANE Select"
    elif s == 'canonical':
        s = "Selected"
    return s

class BAISGeneTranscript(object):
    def __init__(self,seqcache):
        self.seqcache = seqcache

    def gene_shimmer(self,chrom,leaf,type_,dir_):
        path = chrom.file_path("genes_and_transcripts","canonical.bb")
        data = get_bigbed_data(path,chrom.name,leaf.start,leaf.end)
        starts = []
        lens = []
        colour = 1 if type_ == 'pc' else 0
        for line in data:
            gene_start = int(line[0])
            gene_end = int(line[1])
            parts = line[2].split("\t")
            (biotype,gene_name,strand,gene_id) = (parts[16],parts[15],parts[2],parts[14])
            if gene_name == "none":
                gene_name = parts[14]
            if type_ == 'feat':
                colour = 2
                dir_ = ("fwd" if strand == '+' else "rev")
            else:
                if gene_name in FEATURED:
                    continue
                if (strand == '+') != (dir_ == 'fwd'):
                    continue
                if (biotype == 'protein_coding') != (type_ == 'pc'):
                    continue
            starts.append(gene_start)
            lens.append(gene_end-gene_start)
        (starts, lens, senses) = shimmer(starts,lens,True,leaf.start,leaf.end)
        if type_ == 'feat':
            colour = 2
            dir_ = 0
        elif dir_ == 'fwd':
            dir_ = 2
        elif dir_ == 'rev':
            dir_ = 1
        else:
            dir_ = 0
        return ([starts,lens,senses,[colour,dir_]],leaf)

    def gene(self,chrom,leaf,type_,dir_,get_names,focus=None):        
        path = chrom.file_path("genes_and_transcripts","canonical.bb")
        data = get_bigbed_data(path,chrom.name,leaf.start,leaf.end)
        out_starts = []
        out_lens = []
        names = []
        ids = []
        ids_disp = []
        strands = []
        biotypes = []
        prestiges = []
        trans_ids = []
        trans_ids_disp = []
        colour = 1 if type_ == 'pc' else 0
        for line in data:
            gene_start = int(line[0])
            gene_end = int(line[1])
            parts = line[2].split("\t")
            (
                biotype,gene_name,strand,gene_id,prestige,trans_id
            ) = (
                parts[16],parts[15],parts[2],parts[14],parts[18],parts[0]
            )
            disp_id = gene_id
            disp_trans_id = trans_id
            gene_id = id_strip.sub('',gene_id)
            trans_id = id_strip.sub('',trans_id)
            if gene_name == "none":
                gene_name = parts[14]
            if type_ == 'feat':
                colour = 2
                dir_ = ("fwd" if strand == '+' else "rev")
                if focus != gene_id:
                    continue
            else:
                if (strand == '+') != (dir_ == 'fwd'):
                    continue
                if (biotype == 'protein_coding') != (type_ == 'pc'):
                    continue
            out_starts.append(gene_start)
            out_lens.append(gene_end-gene_start)
            ids.append("{0}:gene:{1}".format(chrom.species.wire_genome_id,gene_id))
            strands.append(1+(strand == '+'))
            if get_names:
                names.append(gene_name)
                ids_disp.append(disp_id)
                biotypes.append(munge_code(biotype))
                prestiges.append(munge_code(prestige))
                 # TODO transcript should have trans id but not supported yet
                trans_ids.append("{0}:transcript:{1}".format(chrom.species.wire_genome_id,trans_id))
                trans_ids_disp.append(disp_trans_id)
        if type_ == 'feat':
            colour = 2
            dir_ = 0
        elif dir_ == 'fwd':
            dir_ = 2
        elif dir_ == 'rev':
            dir_ = 1
        else:
            dir_ = 0
        if focus != None:
            print([
                out_starts,out_lens,{ "string": names },[colour,dir_], # 1-4
                { "string": ids },strands,{ "string": biotypes },{ "string": prestiges}, #5-8
                { "string": trans_ids }, { "string": ids_disp }, { "string": trans_ids_disp }], #9-11
                leaf)
        return ([
                out_starts,out_lens,{ "string": names },[colour,dir_], # 1-4
                { "string": ids },strands,{ "string": biotypes },{ "string": prestiges}, #5-8
                { "string": trans_ids }, { "string": ids_disp }, { "string": trans_ids_disp }], #9-11
                leaf)

    def transcript(self,chrom,leaf,type_,dir_,seq,names):
        min_bp = leaf.bp_px / MIN_WIDTH
        path = chrom.file_path("genes_and_transcripts","canonical.bb")
        data = get_bigbed_data(path,chrom.name,leaf.start,leaf.end)
        out_starts = []
        out_lens = []
        out_nump = []
        out_pattern = []
        out_utrs = []
        out_exons = []
        out_introns = []
        seq_req = []
        names = []
        ids = []
        ids_disp = []
        strands = []
        biotypes = []
        prestiges = []
        trans_ids = []
        trans_ids_disp = []
        colour = 1 if type_ == 'pc' else 0
        for line in data:
            gene_start = int(line[0])
            gene_end = int(line[1])
            parts = line[2].split("\t")
            (
                biotype,gene_name,part_starts,part_lens,cds_start,cds_end,
                strand,gene_id,prestige,trans_id
            ) = (
                parts[16],parts[15],parts[8],parts[7],parts[3],parts[4],
                parts[2],parts[14],parts[18],parts[0]
            )
            gene_id = id_strip.sub('',gene_id)
            trans_id = id_strip.sub('',trans_id)
            if gene_name == "none":
                gene_name = parts[14]
            if type_ == 'feat':
                colour = 2
                dir_ = ("fwd" if strand == '+' else "rev")
            else:
                if (strand == '+') != (dir_ == 'fwd'):
                    continue
                if (biotype == 'protein_coding') != (type_ == 'pc'):
                    continue
            seq_req.append((max(gene_start,leaf.start),min(gene_end,leaf.end)))
            names.append(gene_name)
            ids_disp.append(gene_id)
            ids.append("{0}:gene:{1}".format(chrom.species.wire_genome_id,gene_id))
            strands.append((strand == '+')+1)
            biotypes.append(munge_code(biotype))
            prestiges.append(munge_code(prestige))
            trans_ids_disp.append(trans_id)
            trans_ids.append("{0}:transcript:{1}".format(chrom.species.wire_genome_id,trans_id))
            if part_starts.endswith(","): part_starts = part_starts[:-1]
            if part_lens.endswith(","): part_lens = part_lens[:-1]
            part_starts = [int(x) for x in part_starts.split(",")]
            part_lens = [int(x) for x in part_lens.split(",")]
            cds_start = int(cds_start) - gene_start
            cds_end = int(cds_end) - gene_start
            # build basic intron/exon pattern (split, but don't mark UTR)
            blocks = []
            prev_exon_end = 0
            undershoot = 0
            for (exon_start,exon_len) in zip(part_starts,part_lens):
                new_undershoot = max(min_bp-exon_len,0)
                # intron between previous exan and this one
                if exon_start != prev_exon_end:
                    intron_start = prev_exon_end
                    intron_len = exon_start - prev_exon_end
                    if undershoot > 0:
                        stolen = min(undershoot,intron_len)
                        blocks[-1][2] += stolen
                        intron_len -= stolen
                    blocks.append([2,intron_start,intron_len])
                # if 5' is in this exon, split that off now
                if cds_start > exon_start and cds_start < exon_start+exon_len:
                    blocks.append([1,exon_start,cds_start-exon_start])
                    exon_len -= cds_start-exon_start
                    exon_start = cds_start
                # if 3' is in this exon, split of main body now
                if cds_end > exon_start and cds_end < exon_start+exon_len:
                    blocks.append([1,exon_start,cds_end-exon_start])
                    exon_len -= cds_end-exon_start
                    exon_start = cds_end
                # whatever remains of this exon (main or 3')
                blocks.append([1,exon_start,exon_len])
                prev_exon_end = exon_start + exon_len
                undershoot = new_undershoot
            # mark UTRs
            for b in blocks:
                if b[0] == 1 and (b[1] < cds_start or b[1] >= cds_end):
                    b[0] = 0
            # put into output strucutre
            out_starts.append(gene_start)
            out_lens.append(gene_end-gene_start)
            out_nump.append(len(blocks))
            for b in blocks:
                out_pattern.append(b[0])
                if b[0] == 2:
                    out_introns.append(b[2])
                elif b[0] == 1:
                    out_exons.append(b[2])
                else:
                    out_utrs.append(b[2])
        if type_ == 'feat':
            colour = 2
            dir_ = 0
        elif dir_ == 'fwd':
            dir_ = 2
        elif dir_ == 'rev':
            dir_ = 1
        else:
            dir_ = 2
        data = [out_starts,out_nump,out_pattern, #1-3
                out_utrs,out_exons,out_introns,  #4-6
                { "string": names },[colour,dir_],out_lens, #7-9
                { "string": ids },strands,{ "string": biotypes }, #10-12
                { "string": prestiges},{ "string": trans_ids }, #13-14
                { "string": ids_disp},{"string": trans_ids_disp}] #15-16
        if seq:
            (seq_text,seq_starts) = self.seqcache.get(chrom,seq_req)
            data += [{ "string": seq_text },seq_starts] #17-18
        return (data,leaf)

    def add_locales(self,chrom,locales):
        path = chrom.file_path("genes_and_transcripts","canonical.bb")
        chr_len = get_chrom_length(path,chrom.name)
        for (start,end,extra) in get_bigbed_data(path,chrom.name,0,chr_len):
            extra = extra.split("\t")
            id_ = id_strip.sub('',extra[14])
            id_ = "{0}:gene:{1}".format(chrom.species.wire_genome_id,id_)
            middle = int((start+end)/2)
            size = end-start
            locales.add_locale(id_,chrom.stick_name,middle-size,middle+size)

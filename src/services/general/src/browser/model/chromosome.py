import os.path, logging

class Chromosome(object):
    def __init__(self,universe,species,parts,hash_):
        self.logging = universe.logging
        self.config_path = universe.config_path
        self.name = parts[0]
        self.size = int(parts[1])
        self.seq_hash = hash_
        self.genome_id = species.genome_id
        self.stick_name = "{0}:{1}".format(
            species.wire_genome_id,self.name
        )
        self.genome_path = species.genome_id
        self.aliases = []
        # HACK for June
        if self.name.startswith("chr"):
            self.aliases.append("{0}:{1}".format(
                species.wire_genome_id,self.name[3:]
            ))
        universe._add_chrom(self)

    def file_path(self,section,filename):
        path = os.path.join(self.config_path,section,self.genome_path,filename)
        if not os.path.exists(path):
            logging.warn("Missing file {0}".format(path))
        return path

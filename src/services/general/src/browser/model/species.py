import re, os.path, logging

from .chromosome import Chromosome

class Species(object):
    def __init__(self,universe,name,config):
        self.logging = universe.logging
        self.production_name = name
        self.genome_id = config['genome_id']
        self.gca = config['gca']
        self.species = config['species']
        self.dbname = config['dbname']
        self.version = config['version']
        self.wire_genome_id = re.sub(r'\W','_',self.genome_id)
        self.genome_path = self.genome_id
        self.config_path = universe.config_path
        self.chroms = {}
        self._load(universe)
        
    def _load(self,universe):
        hashes = {}
        with open(self.file_path("chrom.hashes")) as f:
            for line in f.readlines():
                parts = line.strip().split("\t")
                hashes[parts[0]] = parts[1]        
        with open(self.file_path("chrom.sizes")) as f:
            for line in f.readlines():
                parts = line.strip().split("\t")
                hash_ = hashes[parts[0]]
                chrom = Chromosome(universe,self,parts,hash_)
                self.chroms[chrom.name] = chrom

    def file_path(self,filename):
        path = os.path.join(self.config_path,"common_files",self.genome_path,filename)
        if not os.path.exists(path):
            logging.warn("Missing file {0}".format(path))
        return path

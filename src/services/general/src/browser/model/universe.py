import os.path, logging, yaml

from .species import Species
from .locale import Locale

class Universe(object):
    def __init__(self,config_path):
        self.config_path = config_path
        self.species = {}
        self.sticks = {}
        self.chroms = []
        self.locale = Locale()
        self.logging = logging.getLogger("general")
        self._load()
        
    def _load(self):
        with open(os.path.join(self.config_path,"common_files","genome_id_info.yml")) as f:
            config = yaml.load(f)
            for (k,v) in config.items():
                s = Species(self,k,v)
                self.species[s.wire_genome_id] = s
                
    def _add_chrom(self,chrom):
        self.sticks[chrom.stick_name] = chrom
        self.chroms.append(chrom)
        for alias in chrom.aliases:
            self.sticks[alias] = chrom

    def get_sticks(self):
        return { k: str(v.size) for (k,v) in self.sticks.items() }
        
    def get_from_stick(self,stick):
        return self.sticks[stick]

    def all_chroms(self):
        return self.chroms

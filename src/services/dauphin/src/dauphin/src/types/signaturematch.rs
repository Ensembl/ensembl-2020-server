use super::argumentmatch::ArgumentMatch;

#[derive(Debug)]
pub struct SignatureMatch {
    args: Vec<ArgumentMatch>
}

impl SignatureMatch {
    pub fn new(args: &Vec<ArgumentMatch>) -> SignatureMatch {
        SignatureMatch {
            args: args.clone()
        }
    }

    pub fn each_argument(&self) -> impl Iterator<Item=&ArgumentMatch> {
        self.args.iter()
    }
}

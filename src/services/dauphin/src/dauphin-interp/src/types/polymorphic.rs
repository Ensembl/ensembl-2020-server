/* 
 *  See the NOTICE file distributed with this work for additional information
 *  regarding copyright ownership.
 *  
 *  Licensed under the Apache License, Version 2.0 (the "License"); you may 
 *  not use this file except in compliance with the License. You may obtain a
 *  copy of the License at http://www.apache.org/licenses/LICENSE-2.0
 *  
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS, WITHOUT 
 *  WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use crate::runtime::{ InterpValue, InterpNatural };

/* Convenience utilities for highly-polymorphic instructions */

pub fn arbitrate_type(dst: &InterpValue, src: &InterpValue, prefer_dst: bool) -> Option<InterpNatural> {
    let src_natural = src.get_natural();
    let dst_natural = dst.get_natural();
    if let InterpNatural::Empty = src_natural { return None; }
    Some(if let InterpNatural::Empty = dst_natural {
        src_natural
    } else {
        if prefer_dst { dst_natural } else { src_natural }
    })
}

// If only there were higher-order type bounds in where clauses!
#[macro_export]
macro_rules! pr_arm {
    (($dst:expr),[$($src:expr),*],$wr:tt,$rd:tt,$func:tt,$arm:ident) => {
        {
            let mut d = $dst.$wr()?;
            $func(&mut d,$(&$src.$rd()?.0),*);
        }
    };

    ($dst:expr,[$($src:expr),*],$wr:tt,$rd:tt,$func:tt,$arm:ident) => {
        {
            let mut d = $dst.$wr()?;
            $func(&mut d,$(&$src.$rd()?.0),*);
            $crate::runtime::InterpValue::$arm(d)
        }
    };

    ([$($src:expr),*],$wr:tt,$rd:tt,$func:tt,$arm:ident) => {
        {
            Some($func($(&$src.$rd()?.0),*))
        }
    };
}

#[macro_export]
macro_rules! polymorphic {
    (($dst:expr),[$($src:expr),*],$natural:expr,$func:tt) => {
        match $natural {
            $crate::runtime::InterpNatural::Empty => { $dst },
            $crate::runtime::InterpNatural::Numbers => $crate::pr_arm!(($dst),[$($src),*],to_numbers,to_rc_numbers,$func,Numbers),
            $crate::runtime::InterpNatural::Indexes => $crate::pr_arm!(($dst),[$($src),*],to_indexes,to_rc_indexes,$func,Indexes),
            $crate::runtime::InterpNatural::Boolean => $crate::pr_arm!(($dst),[$($src),*],to_boolean,to_rc_boolean,$func,Boolean),
            $crate::runtime::InterpNatural::Strings => $crate::pr_arm!(($dst),[$($src),*],to_strings,to_rc_strings,$func,Strings),
            $crate::runtime::InterpNatural::Bytes =>   $crate::pr_arm!(($dst),[$($src),*],to_bytes,to_rc_bytes,$func,Bytes),
        }
    };

    ($dst:expr,[$($src:expr),*],$natural:expr,$func:tt) => {
        match $natural {
            $crate::runtime::InterpNatural::Empty => { $dst },
            $crate::runtime::InterpNatural::Numbers => $crate::pr_arm!($dst,[$($src),*],to_numbers,to_rc_numbers,$func,Numbers),
            $crate::runtime::InterpNatural::Indexes => $crate::pr_arm!($dst,[$($src),*],to_indexes,to_rc_indexes,$func,Indexes),
            $crate::runtime::InterpNatural::Boolean => $crate::pr_arm!($dst,[$($src),*],to_boolean,to_rc_boolean,$func,Boolean),
            $crate::runtime::InterpNatural::Strings => $crate::pr_arm!($dst,[$($src),*],to_strings,to_rc_strings,$func,Strings),
            $crate::runtime::InterpNatural::Bytes =>   $crate::pr_arm!($dst,[$($src),*],to_bytes,to_rc_bytes,$func,Bytes),
        }
    };

    ([$($src:expr),*],$natural:expr,$func:tt) => {
        match $natural {
            $crate::runtime::InterpNatural::Empty => { None },
            $crate::runtime::InterpNatural::Numbers => $crate::pr_arm!([$($src),*],to_numbers,to_rc_numbers,$func,Numbers),
            $crate::runtime::InterpNatural::Indexes => $crate::pr_arm!([$($src),*],to_indexes,to_rc_indexes,$func,Indexes),
            $crate::runtime::InterpNatural::Boolean => $crate::pr_arm!([$($src),*],to_boolean,to_rc_boolean,$func,Boolean),
            $crate::runtime::InterpNatural::Strings => $crate::pr_arm!([$($src),*],to_strings,to_rc_strings,$func,Strings),
            $crate::runtime::InterpNatural::Bytes =>   $crate::pr_arm!([$($src),*],to_bytes,to_rc_bytes,$func,Bytes),
        }
    };
}

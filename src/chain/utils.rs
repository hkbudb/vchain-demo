use super::{Parameter, SetElementType};
use crate::acc::{
    self,
    curve::{G1Affine, G2Affine},
    Accumulator,
};

use crate::set::MultiSet;
pub fn multiset_to_g1(set: &MultiSet<SetElementType>, param: &Parameter) -> G1Affine {
    match (param.acc_type, param.use_sk) {
        (acc::Type::ACC1, true) => acc::Acc1::cal_acc_g1_sk(&set),
        (acc::Type::ACC1, false) => acc::Acc1::cal_acc_g1(&set),
        (acc::Type::ACC2, true) => acc::Acc2::cal_acc_g1_sk(&set),
        (acc::Type::ACC2, false) => acc::Acc2::cal_acc_g1(&set),
    }
}

pub fn multiset_to_g2(set: &MultiSet<SetElementType>, param: &Parameter) -> G2Affine {
    match (param.acc_type, param.use_sk) {
        (acc::Type::ACC1, true) => acc::Acc1::cal_acc_g2_sk(&set),
        (acc::Type::ACC1, false) => acc::Acc1::cal_acc_g2(&set),
        (acc::Type::ACC2, true) => acc::Acc2::cal_acc_g2_sk(&set),
        (acc::Type::ACC2, false) => acc::Acc2::cal_acc_g2(&set),
    }
}

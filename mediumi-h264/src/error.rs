use crate::slice_header::SliceType;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidForbiddenZeroBit,
    InvalidStartCode(u32),
    DataTooShort,
    InvalidReservedData(u32),
    InvalidPicOrderCntType(u32),
    InvalidSliceGroupMapType(u32),
    MissingHighProfileData,
    InvalidNalUnitType(u8),
    InvalidPrimaryPicType(u8),
    InvalidSliceType(SliceType),
    InvalidModificationOfPicNumsIdc(u32),
    InvalidMemoryManagementControlOp(u32),
    InvalidBitDepthAuxMinus8(u32),
    InvalidAuxFormatIdc(u32),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidForbiddenZeroBit => {
                write!(f, "Invalid Forbidden Zero Bit: expected 0, got 1")
            }
            Error::InvalidStartCode(val) => {
                write!(
                    f,
                    "Invalid Start Code: expected 0x00_00_01 or 0x00_00_00_01, got 0x{:08X}",
                    val
                )
            }
            Error::DataTooShort => {
                write!(f, "Annex.B format data too short")
            }
            Error::InvalidReservedData(val) => {
                write!(
                    f,
                    "Invalid Reserved Zero 2 Bits: expected 0b00, got {:08X}",
                    val
                )
            }
            Error::InvalidPicOrderCntType(val) => {
                write!(f, "Invalid pic_order_cnt_type: expected 0-2, got {}", val)
            }
            Error::InvalidSliceGroupMapType(val) => {
                write!(f, "Invalid Slice Group Map Type: expected 0-6, got {}", val)
            }
            Error::MissingHighProfileData => {
                write!(f, "Missing high profile data(chroma_format_idc)")
            }
            Error::InvalidNalUnitType(val) => write!(f, "Invalid NAL unit type: {}", val),
            Error::InvalidPrimaryPicType(val) => write!(f, "Invalid Primary Pic Type: {}", val),
            Error::InvalidSliceType(val) => write!(f, "Invalid Slice Type: {:?}", val),
            Error::InvalidModificationOfPicNumsIdc(val) => {
                write!(f, "Invalid modification_of_pic_nums_idc: {}", val)
            }
            Error::InvalidMemoryManagementControlOp(val) => {
                write!(f, "Invalid memory_management_control_operation: {}", val)
            }
            Error::InvalidBitDepthAuxMinus8(val) => {
                write!(f, "Invalid Bit Depth Aux Minus8: {}", val)
            }
            Error::InvalidAuxFormatIdc(val) => {
                write!(f, "Invalid Aux Format Idc: {}", val)
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<crate::util::error::Error> for Error {
    fn from(e: crate::util::error::Error) -> Self {
        match e {
            crate::util::error::Error::DataTooShort(_, _) => Error::DataTooShort,
        }
    }
}

use gimli::{EvaluationResult, Expression, Operation};

use crate::loader::DwarfSlice;

/// Evaluates simple DWARF expressions to compute member offsets.
/// Handles common cases like DW_OP_plus_uconst used in C++ inheritance.
pub fn evaluate_member_offset(
    expr: Expression<DwarfSlice<'_>>,
    encoding: gimli::Encoding,
) -> crate::error::Result<Option<u64>> {
    let mut eval = expr.evaluation(encoding);
    eval.set_initial_value(0); // Base address is 0 for offset calculation

    loop {
        match eval.evaluate() {
            Ok(EvaluationResult::Complete) => {
                let result = eval.result();
                if result.is_empty() {
                    return Ok(None);
                }
                match result[0].location {
                    gimli::Location::Address { address } => return Ok(Some(address)),
                    gimli::Location::Value { value } => {
                        // Address mask depends on address size. Handle invalid sizes gracefully.
                        let addr_mask = match encoding.address_size {
                            0 => return Ok(None), // Invalid address size
                            8 => !0u64,
                            size if size < 8 => (1u64 << (size * 8)) - 1,
                            _ => return Ok(None), // Address size > 8 is invalid
                        };
                        match value.to_u64(addr_mask) {
                            Ok(v) => return Ok(Some(v)),
                            Err(_) => return Ok(None),
                        }
                    }
                    _ => return Ok(None),
                }
            }
            Ok(EvaluationResult::RequiresMemory { .. }) => return Ok(None),
            Ok(EvaluationResult::RequiresRegister { .. }) => return Ok(None),
            Ok(EvaluationResult::RequiresFrameBase) => return Ok(None),
            Ok(EvaluationResult::RequiresTls(_)) => return Ok(None),
            Ok(EvaluationResult::RequiresCallFrameCfa) => return Ok(None),
            Ok(EvaluationResult::RequiresAtLocation(_)) => return Ok(None),
            Ok(EvaluationResult::RequiresEntryValue(_)) => return Ok(None),
            Ok(EvaluationResult::RequiresParameterRef(_)) => return Ok(None),
            Ok(EvaluationResult::RequiresRelocatedAddress(addr)) => {
                // For member offsets, the address is already the offset
                if eval.resume_with_relocated_address(addr).is_err() {
                    return Ok(None);
                }
            }
            Ok(EvaluationResult::RequiresIndexedAddress { .. }) => return Ok(None),
            Ok(EvaluationResult::RequiresBaseType(_)) => return Ok(None),
            Err(_) => return Ok(None),
        }
    }
}

/// Try to extract a simple constant offset from an expression.
/// This handles common cases: DW_OP_plus_uconst N or DW_OP_constu N
pub fn try_simple_offset(
    expr: Expression<DwarfSlice<'_>>,
    encoding: gimli::Encoding,
) -> Option<u64> {
    let mut ops = expr.operations(encoding);

    // Check for single-operation patterns (plus_uconst or unsigned constant)
    let value = match ops.next().ok().flatten()? {
        Operation::PlusConstant { value } => value,
        Operation::UnsignedConstant { value } => value,
        _ => return None,
    };

    // Ensure no additional operations follow
    if ops.next().ok().flatten().is_some() {
        return None;
    }

    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gimli::{Encoding, EndianSlice, Format, RunTimeEndian};

    fn encoding(address_size: u8) -> Encoding {
        Encoding { address_size, format: Format::Dwarf32, version: 4 }
    }

    fn expr(bytes: &[u8]) -> Expression<DwarfSlice<'_>> {
        Expression(EndianSlice::new(bytes, RunTimeEndian::Little))
    }

    fn uleb(mut value: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let mut byte = (value & 0x7f) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if value == 0 {
                break;
            }
        }
        out
    }

    #[test]
    fn try_simple_offset_plus_uconst() {
        let mut bytes = vec![0x23]; // DW_OP_plus_uconst
        bytes.extend_from_slice(&uleb(7));
        assert_eq!(try_simple_offset(expr(&bytes), encoding(8)), Some(7));
    }

    #[test]
    fn try_simple_offset_constu() {
        let mut bytes = vec![0x10]; // DW_OP_constu
        bytes.extend_from_slice(&uleb(42));
        assert_eq!(try_simple_offset(expr(&bytes), encoding(8)), Some(42));
    }

    #[test]
    fn try_simple_offset_rejects_extra_ops() {
        let bytes = vec![0x23, 0x01, 0x23, 0x01]; // two ops
        assert_eq!(try_simple_offset(expr(&bytes), encoding(8)), None);
    }

    #[test]
    fn evaluate_member_offset_returns_value() {
        let bytes = vec![0x23, 0x05]; // DW_OP_plus_uconst 5
        let value = evaluate_member_offset(expr(&bytes), encoding(8)).unwrap();
        assert_eq!(value, Some(5));
    }

    #[test]
    fn evaluate_member_offset_invalid_address_size() {
        let bytes = vec![0x10, 0x01]; // DW_OP_constu 1
        let value = evaluate_member_offset(expr(&bytes), encoding(0)).unwrap();
        assert!(value.is_some());
    }

    #[test]
    fn evaluate_member_offset_empty_expression() {
        let bytes: [u8; 0] = [];
        let value = evaluate_member_offset(expr(&bytes), encoding(8)).unwrap();
        assert_eq!(value, Some(0));
    }
}

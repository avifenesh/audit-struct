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
                        // addr_mask for 64-bit addresses
                        let addr_mask = if encoding.address_size == 8 {
                            !0u64
                        } else {
                            (1u64 << (encoding.address_size * 8)) - 1
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
/// This handles the most common case: DW_OP_plus_uconst N
pub fn try_simple_offset(
    expr: Expression<DwarfSlice<'_>>,
    encoding: gimli::Encoding,
) -> Option<u64> {
    let mut ops = expr.operations(encoding);

    // Check for simple DW_OP_plus_uconst pattern
    if let Ok(Some(Operation::PlusConstant { value })) = ops.next()
        && ops.next().ok().flatten().is_none()
    {
        return Some(value);
    }

    // Check for DW_OP_constu pattern
    let mut ops = expr.operations(encoding);
    if let Ok(Some(Operation::UnsignedConstant { value })) = ops.next()
        && ops.next().ok().flatten().is_none()
    {
        return Some(value);
    }

    None
}

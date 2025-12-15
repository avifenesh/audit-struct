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

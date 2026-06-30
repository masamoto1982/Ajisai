"""User-level error categories (Section 11.1).

A raised error propagates and halts the current evaluation (Section 11.4).
Ajisai has no construct that turns a raised error into a value; partial failure
of a *well-formed* operation is the Bubble Rule's job (Section 11.2) and yields
NIL, not an exception.
"""


class AjisaiError(Exception):
    category = "Custom"

    def __init__(self, message=""):
        super().__init__(message)
        self.message = message


class StackUnderflow(AjisaiError):
    category = "StackUnderflow"


class StructureError(AjisaiError):
    category = "StructureError"


class UnknownWord(AjisaiError):
    category = "UnknownWord"


class UnknownModule(AjisaiError):
    category = "UnknownModule"


class DivisionByZero(AjisaiError):
    category = "DivisionByZero"


class IndexOutOfBounds(AjisaiError):
    category = "IndexOutOfBounds"


class VectorLengthMismatch(AjisaiError):
    category = "VectorLengthMismatch"


class ExecutionLimitExceeded(AjisaiError):
    category = "ExecutionLimitExceeded"


class ModeUnsupported(AjisaiError):
    category = "ModeUnsupported"


class BuiltinProtection(AjisaiError):
    category = "BuiltinProtection"


class CondExhausted(AjisaiError):
    category = "CondExhausted"


class CustomError(AjisaiError):
    category = "Custom"


class TokenizerError(AjisaiError):
    category = "TokenizerError"

"""Shared SAC contract primitives and producer-error translation."""
from __future__ import annotations

import re

from .render_contract_validation import (
    RenderContractError as RenderContractValidationError,
    validate_render_contract as _validate_render_contract,
    validate_style_reference_separation as _validate_style_reference_separation,
)


class ContractError(ValueError):
    pass


_SHA256_RE = re.compile(r'^[0-9a-f]{64}$')
PROMPT_PROFILES = {'PROJECT_STYLE','PHOTOREAL_SCENE','PHOTOREAL_CHARACTER_ANCHOR','PHOTOREAL_LAYER_ASSET','CUSTOM'}
REUSABLE_TRANSPARENT_KINDS = frozenset({'CHARACTER_ANCHOR','CHARACTER_ASSET','PROP_ANCHOR','PROP_ASSET'})


def req(obj, key, typ=None):
    if key not in obj:
        raise ContractError(f'MISSING:{key}')
    value = obj[key]
    if typ is not None and not isinstance(value, typ):
        raise ContractError(f'TYPE:{key}')
    return value


def reject_unknown(obj, allowed, where):
    extra = set(obj) - set(allowed)
    if extra:
        raise ContractError(f'UNKNOWN_FIELDS:{where}:{sorted(extra)}')


def validate_render_contract(o, where='render_contract'):
    try:
        return _validate_render_contract(o, where)
    except RenderContractValidationError as error:
        raise ContractError(str(error)) from error


def validate_style_reference_separation(references, where='references'):
    try:
        return _validate_style_reference_separation(references, where)
    except RenderContractValidationError as error:
        raise ContractError(str(error)) from error

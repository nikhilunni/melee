/*
 * Host shim for the decomp's placeholder.h.
 *
 * lbtrigf.c wraps atanf in `#ifdef __MWERKS__` (the decomp only builds it
 * with the retail compiler). lbtrigf.c includes this header before that
 * check, so defining the macro here brings atanf into the oracle build
 * without exposing the host's system headers to it (driver.c includes them
 * before it includes the decomp sources).
 */
#ifndef MELEE_LB_REF_SHIM_PLACEHOLDER_H
#define MELEE_LB_REF_SHIM_PLACEHOLDER_H

#ifndef __MWERKS__
#define __MWERKS__ 1
#endif

#endif

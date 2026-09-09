/* Compile the actual Wait IASA caller. The leaf predicates record visitation
 * and return the selected match; transition bodies are deliberately absent. */
#include <stdio.h>
#include <stdbool.h>
typedef struct { unsigned mask; } Fighter_GObj;
#define RETURN_IF(expr) do { if (expr) return; } while (0)
#define PREDICATE(name, id) static bool name(Fighter_GObj *g) { \
    putchar(id); return (g->mask & (1u << (id))) != 0; }
PREDICATE(ftCo_SpecialS_CheckInput, 0)
PREDICATE(ftCo_Attack100_CheckInput, 1)
PREDICATE(ftCo_800D6824, 2)
PREDICATE(ftCo_800D68C0, 3)
PREDICATE(ftCo_Catch_CheckInput, 4)
PREDICATE(ftCo_AttackS4_CheckInput, 5)
PREDICATE(ftCo_AttackHi4_CheckInput, 6)
PREDICATE(ftCo_AttackLw4_CheckInput, 7)
PREDICATE(ftCo_AttackS3_CheckInput, 8)
PREDICATE(ftCo_AttackHi3_CheckInput, 9)
PREDICATE(ftCo_AttackLw3_CheckInput, 10)
PREDICATE(ftCo_Attack1_CheckInput, 11)
PREDICATE(ftCo_80099794, 12)
PREDICATE(ftCo_80091A4C, 13)
PREDICATE(ftFx_AppealS_CheckInput, 14)
PREDICATE(ftCo_800DE9D8, 15)
PREDICATE(ftCo_Jump_CheckInput, 16)
PREDICATE(ftCo_Dash_CheckInput, 17)
PREDICATE(ftCo_800D5FB0, 18)
PREDICATE(ftCo_Turn_CheckInput, 19)
PREDICATE(ftCo_Walk_CheckInput, 20)
#include "wait.c"
int main(void) {
    Fighter_GObj g = {0};
    ftCo_Wait_IASA(&g); putchar(255);
    for (unsigned a = 0; a < 21; ++a) {
        for (unsigned b = 0; b < 21; ++b) {
            g.mask = (1u << a) | (1u << b);
            ftCo_Wait_IASA(&g); putchar(255);
        }
    }
    return ferror(stdout) != 0;
}

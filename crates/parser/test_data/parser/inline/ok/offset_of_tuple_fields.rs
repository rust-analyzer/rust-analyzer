fn foo() {
    builtin#offset_of(ComplexTup, 0.1);
    builtin#offset_of(ComplexTup, 0.1.1.1);
    builtin#offset_of(ComplexTup, 0. 1);
    builtin#offset_of(ComplexTup, 0 .1.1.1);
}

function eval_ruby_script_int(text) {
    return Module.ccall(
        'eval_ruby_script_int',
        'number',
        ['string'],
        [text]
    );
}

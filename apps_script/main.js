/**
 * eval ruby script and return integer value
 * @param {string} input ruby code.
 * @return The input multiplied by 2.
 * @customfunction
*/
function EVAL_RUBY_SCRIPT_INT(text) {
    return Module.ccall(
        'eval_ruby_script_int',
        'number',
        ['string'],
        [text]
    );
}

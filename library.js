mergeInto(LibraryManager.library, {
    systemMessage: function (msgPtr) {
        const msg = UTF8ToString(msgPtr);
        if (Module.systemMessage) {
            Module.systemMessage(msgPtr);
        } else {
            console.info('[SYSTEM]', msg);
        }
    }
});

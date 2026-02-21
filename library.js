mergeInto(LibraryManager.library, {
    systemMessage: function (msgPtr) {
        const msg = UTF8ToString(msgPtr);
        if (Module.systemMessage) {
            Module.systemMessage(msgPtr);
        } else {
            console.info('[SYSTEM]', msg);
        }
    },

    getTimeSec: function () {
        return BigInt(Math.floor(Date.now() / 1000));
    },

    getTimeNanosec: function () {
        return (Date.now() % 1000) * 1000000;
    },

    getOffset: function () {
        return new Date().getTimezoneOffset() * -60;
    },
});

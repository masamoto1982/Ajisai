












const AjisaiConfig = {
    // App UI version shown in headers (edit this value when you want to bump the displayed version)
    appVersion: '202604102001',














    primaryColor: '#6b5b95',







    meta: {
        title: "Ajisai",
        subTitle: "FORTH-inspired Stack-based Language",
        copyrightYear: new Date().getFullYear()
    },




    project: {
        name: "Ajisai Programming Language",
        shortName: "Ajisai",
        author: "masamoto yamashiro",
        url: "https://masamoto1982.github.io/Ajisai/",
        repository: "https://github.com/masamoto1982/Ajisai"
    },




    globalMenu: [
        { label: "Home", link: "index.html" },
        { label: "Philosophy", link: "philosophy.html" },
        { label: "About", link: "about.html" },
        { label: "Tutorial", link: "tutorial.html" }
    ],

    serviceMenu: [
        { label: "Syntax", link: "syntax.html" },
        { label: "Built-in Words", link: "words.html" },
        { label: "Data Types", link: "types.html" },
        { label: "Control Flow", link: "control.html" },
        { label: "Higher-Order", link: "higher-order.html" }
    ],

    referenceMenu: [
        { label: "Examples", link: "examples.html" },
        { label: "GitHub", link: "https://github.com/masamoto1982/Ajisai" },
        { label: "Demo", link: "https://masamoto1982.github.io/Ajisai/" }
    ],




    social: {
        github: { url: "https://github.com/masamoto1982/Ajisai", label: "GitHub" },
        demo: { url: "https://masamoto1982.github.io/Ajisai/", label: "Try Demo" }
    }
};


if (typeof window !== 'undefined') {
    window.AjisaiConfig = AjisaiConfig;
}

-- vim.lsp.log.set_level(vim.lsp.log_levels.DEBUG)

vim.lsp.config("neorg-ls", {
    cmd = { vim.fs.normalize("~/projects/neorg-ls/target/debug/neorg-language-server") },
    root_markers = { "root.toml" },
    filetypes = { "norg" },
})

vim.lsp.enable("neorg-ls")

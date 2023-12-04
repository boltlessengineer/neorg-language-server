-- vim.lsp.set_log_level(vim.lsp.log_levels.DEBUG)
-- local client_id = vim.lsp.start({
-- 	name = "neorg-ls",
-- 	cmd = { "./target/debug/neorg-language-server" },
-- 	root_dir = vim.fs.dirname(vim.fs.find({ "index.norg" }, { upward = true })[1]),
-- })
-- vim.notify("server:" .. client_id)

require("lsp-debug-tools").start({
	expected = { "norg" },
	name = "neorg-ls",
	cmd = { "./target/debug/neorg-language-server" },
	root_dir = vim.fs.dirname(vim.fs.find({ "index.norg" }, { upward = true })[1]),
})

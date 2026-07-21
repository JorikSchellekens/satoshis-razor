namespace Razor.Audit

/-- End-user audit test: a trivial arithmetic fact, submitted to exercise the
remote submission pipeline. -/
theorem n_add_zero : ∀ n : Nat, n + 0 = n := fun n => Nat.add_zero n

end Razor.Audit

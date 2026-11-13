(function() {
    const implementors = Object.fromEntries([["rust_analyzer",[["impl&lt;S, W&gt; Layer&lt;S&gt; for <a class=\"struct\" href=\"rust_analyzer/tracing/json/struct.TimingLayer.html\" title=\"struct rust_analyzer::tracing::json::TimingLayer\">TimingLayer</a>&lt;S, W&gt;<div class=\"where\">where\n    S: Subscriber + for&lt;'span&gt; LookupSpan&lt;'span&gt;,\n    W: for&lt;'writer&gt; MakeWriter&lt;'writer&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.100.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.100.0/core/marker/trait.Sync.html\" title=\"trait core::marker::Sync\">Sync</a> + 'static,</div>",0],["impl&lt;S&gt; Layer&lt;S&gt; for <a class=\"struct\" href=\"rust_analyzer/tracing/hprof/struct.SpanTree.html\" title=\"struct rust_analyzer::tracing::hprof::SpanTree\">SpanTree</a>&lt;S&gt;<div class=\"where\">where\n    S: Subscriber + for&lt;'span&gt; LookupSpan&lt;'span&gt;,</div>",0]]]]);
    if (window.register_implementors) {
        window.register_implementors(implementors);
    } else {
        window.pending_implementors = implementors;
    }
})()
//{"start":59,"fragment_lengths":[950]}
pub async fn wait() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate());
        if let Ok(mut terminate) = terminate {
            tokio::select! {
                _ = ctrl_c => {}
                _ = terminate.recv() => {}
            }
            return;
        }
    }

    let _ = ctrl_c.await;
}

pub async fn requested(mut receiver: tokio::sync::watch::Receiver<bool>) {
    let _ = receiver.wait_for(|shutdown| *shutdown).await;
}

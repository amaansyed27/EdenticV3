const now = new Date();

export const demoBootstrap = {
  settings: {
    onboardingComplete: true,
    projectsRoot: "C:\\Users\\Amaan\\Videos\\Edentic Projects",
    theme: "dark",
    computeMode: "auto",
    proxyQuality: "balanced",
    cacheLimitGb: 40,
    maxConcurrentJobs: 2,
    whisperModel: "small",
    openrouterConfigured: true,
    openrouterModel: "openrouter/free",
  },
  hardware: {
    gpuName: "NVIDIA GeForce RTX 5060 Laptop GPU",
    ffmpegVersion: "FFmpeg 7.1",
    ffprobeVersion: "ffprobe 7.1",
    pythonVersion: "Python 3.12.10",
  },
  projects: [
    {
      id: "project-winreclaim",
      name: "WinReclaim Launch",
      path: "C:\\Users\\Amaan\\Videos\\Edentic Projects\\WinReclaim Launch",
      createdAt: new Date(now - 86400000 * 3).toISOString(),
      updatedAt: new Date(now - 1000 * 60 * 18).toISOString(),
      aspectRatio: "16:9",
      resolution: "1920x1080",
      frameRate: 30,
      thumbnailPath: "",
      assetCount: 3,
    },
    {
      id: "project-f1",
      name: "Monaco Race Edit",
      path: "C:\\Users\\Amaan\\Videos\\Edentic Projects\\Monaco Race Edit",
      createdAt: new Date(now - 86400000 * 8).toISOString(),
      updatedAt: new Date(now - 86400000 * 2).toISOString(),
      aspectRatio: "9:16",
      resolution: "1080x1920",
      frameRate: 60,
      thumbnailPath: "",
      assetCount: 8,
    },
  ],
};

export const demoProject = {
  project: demoBootstrap.projects[0],
  assets: [
    {
      id: "asset-demo",
      projectId: "project-winreclaim",
      name: "winreclaim-demo-recording.mp4",
      originalPath: "C:\\Videos\\winreclaim-demo-recording.mp4",
      managedPath: "C:\\Users\\Amaan\\Videos\\Edentic Projects\\WinReclaim Launch\\Media\\Originals\\winreclaim-demo-recording.mp4",
      duration: 194.6,
      width: 1920,
      height: 1080,
      frameRate: 30,
      sizeBytes: 184200000,
      videoCodec: "h264",
      audioCodec: "aac",
      proxyPath: "",
      posterPath: "",
      waveformPath: "",
      indexStatus: "ready",
    },
  ],
  scenes: [
    { id: "s1", assetId: "asset-demo", start: 0, end: 18.4, label: "Opening dashboard and scan", thumbnailPath: "" },
    { id: "s2", assetId: "asset-demo", start: 18.4, end: 46.1, label: "Storage findings overview", thumbnailPath: "" },
    { id: "s3", assetId: "asset-demo", start: 46.1, end: 84.8, label: "Reviewing safe cleanup items", thumbnailPath: "" },
    { id: "s4", assetId: "asset-demo", start: 84.8, end: 133.2, label: "Running the cleanup", thumbnailPath: "" },
    { id: "s5", assetId: "asset-demo", start: 133.2, end: 194.6, label: "Cleanup receipt and results", thumbnailPath: "" },
  ],
  transcript: [
    { id: "t1", assetId: "asset-demo", start: 4.2, end: 10.8, text: "This is WinReclaim, a safer way to understand what is filling your Windows drive." },
    { id: "t2", assetId: "asset-demo", start: 26.0, end: 34.7, text: "The scan separates disposable caches from projects, local models and personal files." },
    { id: "t3", assetId: "asset-demo", start: 59.4, end: 68.1, text: "Every item explains what removing it means and whether it needs to be downloaded again." },
    { id: "t4", assetId: "asset-demo", start: 145.0, end: 156.2, text: "The cleanup receipt shows exactly what changed and how much storage was recovered." },
  ],
  contexts: [
    { id: "context-1", name: "Implementation context", source: "pasted", createdAt: now.toISOString(), content: "WinReclaim is a local-first Windows storage intelligence tool." },
  ],
  jobs: [],
};

<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';

  // 状态变量
  let audioContext: AudioContext | null = null;
  let analyser: AnalyserNode | null = null;
  let audioElement: HTMLAudioElement | null = null;
  let animationFrameId: number | null = null;
  let bars: number[] = Array(64).fill(0);
  let currentSong = $state({ title: 'Loading...', artist: '' });
  let themeColor = $state('#3498db');
  let isPinned = $state(true);
  let isMousePassthrough = $state(false);
  let streamUrl = 'https://radio.startend.xyz/radio';
  
  // 获取当前窗口实例
  const appWindow = getCurrentWindow();
  
  // 初始化音频上下文
  function initAudio() {
    try {
      audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();
      analyser = audioContext.createAnalyser();
      analyser.fftSize = 256;
      
      // 创建音频元素
      audioElement = new Audio();
      audioElement.src = streamUrl;
      audioElement.crossOrigin = 'anonymous';
      
      // 连接音频节点
      const source = audioContext.createMediaElementSource(audioElement);
      source.connect(analyser);
      analyser.connect(audioContext.destination);
      
      // 开始播放
      audioElement.play().catch(e => console.error('播放失败:', e));
      
      // 开始可视化更新
      updateVisualization();
    } catch (error) {
      console.error('音频初始化失败:', error);
    }
  }
  
  // 更新可视化
  function updateVisualization() {
    if (!analyser) return;
    
    const dataArray = new Uint8Array(analyser.frequencyBinCount);
    analyser.getByteFrequencyData(dataArray);
    
    // 更新条形图数据
    bars = Array.from(dataArray).slice(0, 64);
    
    // 继续下一帧
    animationFrameId = requestAnimationFrame(updateVisualization);
  }
  
  // 获取当前播放歌曲信息
  async function fetchCurrentSong() {
    try {
      // 这里使用轮询方式获取歌曲信息
      // 您也可以根据需要改为 SSE 方式
      const response = await fetch('https://radio.startend.xyz/status-json.xsl');
      const data = await response.json();
      
      if (data.icestats && data.icestats.source && data.icestats.source.length > 0) {
        const source = data.icestats.source[0];
        currentSong = {
          title: source.title || 'Unknown Title',
          artist: source.artist || 'Unknown Artist'
        };
      }
    } catch (error) {
      console.error('获取歌曲信息失败:', error);
    }
  }
  
  // 设置窗口置顶
  async function setAlwaysOnTop(flag: boolean) {
    isPinned = flag;
    await appWindow.setAlwaysOnTop(flag);
    // 调用 Rust 命令同步状态
    await invoke('set_always_on_top', { flag });
  }
  
  // 设置鼠标穿透
  async function setMousePassthrough(flag: boolean) {
    isMousePassthrough = flag;
    await appWindow.setIgnoreCursorEvents(flag);
    // 调用 Rust 命令同步状态
    await invoke('set_mouse_passthrough', { flag });
  }
  
  // 更改主题颜色
  async function changeThemeColor(color: string) {
    themeColor = color;
    // 调用 Rust 命令
    await invoke('change_theme_color', { color });
  }
  
  // 移动窗口（用于无边框窗口）
  function startDrag() {
    appWindow.startDragging();
  }
  
  // 最小化窗口
  async function minimizeWindow() {
    await appWindow.hide();
  }
  
  // 组件挂载时
  onMount(() => {
    // 初始化音频
    initAudio();
    
    // 设置窗口置顶
    setAlwaysOnTop(true);
    
    // 定期获取歌曲信息
    fetchCurrentSong();
    const songInterval = setInterval(fetchCurrentSong, 10000); // 每10秒获取一次
    
    // 返回清理函数
    return () => {
      clearInterval(songInterval);
      if (animationFrameId) {
        cancelAnimationFrame(animationFrameId);
      }
      if (audioElement) {
        audioElement.pause();
      }
      if (audioContext) {
        audioContext.close();
      }
    };
  });
</script>

<main class="container" on:mousedown={startDrag}>
  <div class="header">
    <div class="controls">
      <button class="control-btn" on:click={() => setAlwaysOnTop(!isPinned)}>
        {isPinned ? '🔓' : '🔒'}
      </button>
      <button class="control-btn" on:click={() => setMousePassthrough(!isMousePassthrough)}>
        {isMousePassthrough ? '🖱️' : '✋'}
      </button>
      <button class="control-btn" on:click={minimizeWindow}>−</button>
    </div>
  </div>
  
  <div class="visualization">
    {#each bars as bar, i}
      <div 
        class="bar" 
        style="height: {Math.max(bar / 2, 2)}px; background-color: {themeColor};"
      ></div>
    {/each}
  </div>
  
  <div class="song-info">
    <h2>{currentSong.title}</h2>
    <p>{currentSong.artist}</p>
  </div>
</main>

<style>
  :global(body) {
    background-color: transparent !important;
    overflow: hidden;
    margin: 0;
    padding: 0;
  }
  
  :global(html) {
    background-color: transparent !important;
  }
  
  .container {
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(10px);
    border-radius: 10px;
    padding: 10px;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    cursor: move;
    -webkit-app-region: drag;
  }
  
  .header {
    display: flex;
    justify-content: flex-end;
    margin-bottom: 5px;
  }
  
  .controls {
    display: flex;
    gap: 5px;
    -webkit-app-region: no-drag;
  }
  
  .control-btn {
    background: rgba(255, 255, 255, 0.2);
    border: none;
    border-radius: 4px;
    color: white;
    width: 24px;
    height: 24px;
    cursor: pointer;
    font-size: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  
  .control-btn:hover {
    background: rgba(255, 255, 255, 0.3);
  }
  
  .visualization {
    display: flex;
    align-items: end;
    justify-content: center;
    gap: 2px;
    height: 60px;
    margin-bottom: 10px;
  }
  
  .bar {
    width: 4px;
    border-radius: 2px 2px 0 0;
    transition: height 0.1s ease;
    -webkit-app-region: no-drag;
  }
  
  .song-info {
    text-align: center;
    color: white;
    -webkit-app-region: no-drag;
  }
  
  .song-info h2 {
    margin: 0 0 3px 0;
    font-size: 16px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  
  .song-info p {
    margin: 0;
    font-size: 14px;
    opacity: 0.8;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
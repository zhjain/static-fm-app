<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  // 定义歌曲信息类型
  interface SongInfo {
    title: string;
    artist: string;
  }

  // 状态变量
  let audioContext: AudioContext | null = null;
  let analyser: AnalyserNode | null = null;
  let audioElement: HTMLAudioElement | null = null;
  let animationFrameId: number | null = null;
  let bars = $state(Array(64).fill(0));
  let currentSong = $state({ title: 'Loading...', artist: '' });
  let themeColor = $state('#3498db');
  let isPinned = $state(true);
  let isMousePassthrough = $state(false);
  let streamUrl = 'https://radio.startend.xyz/radio';
  let isPlaying = $state(false);
  
  // 获取当前窗口实例
  const appWindow = getCurrentWindow();
  
  // 初始化音频上下文
  async function initAudio() {
    try {
      // 如果已有音频上下文，先清理
      if (audioContext) {
        await audioContext.close();
      }
      
      audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();
      analyser = audioContext.createAnalyser();
      analyser.fftSize = 256;
      analyser.smoothingTimeConstant = 0.8;
      
      // 创建音频元素
      if (audioElement) {
        audioElement.pause();
        audioElement = null;
      }
      
      audioElement = new Audio();
      audioElement.src = streamUrl;
      audioElement.crossOrigin = 'anonymous';
      audioElement.volume = 0.8;
      
      // 添加事件监听器
      audioElement.addEventListener('error', (e) => {
        console.error('音频播放错误:', e);
        isPlaying = false;
      });
      
      audioElement.addEventListener('playing', () => {
        isPlaying = true;
      });
      
      audioElement.addEventListener('pause', () => {
        isPlaying = false;
      });
      
      // 连接音频节点
      const source = audioContext.createMediaElementSource(audioElement);
      source.connect(analyser);
      analyser.connect(audioContext.destination);
      
      // 开始播放
      await audioContext.resume();
      const playPromise = audioElement.play();
      if (playPromise !== undefined) {
        playPromise.catch(e => {
          console.error('播放失败:', e);
          isPlaying = false;
        });
      }
      
      // 开始可视化更新
      updateVisualization();
    } catch (error) {
      console.error('音频初始化失败:', error);
      isPlaying = false;
    }
  }
  
  // 更新可视化
  function updateVisualization() {
    if (!analyser || !audioContext) return;
    
    // 检查音频上下文状态
    if (audioContext.state === 'suspended') {
      audioContext.resume();
    }
    
    const dataArray = new Uint8Array(analyser.frequencyBinCount);
    analyser.getByteFrequencyData(dataArray);
    
    // 更新条形图数据
    bars = Array.from(dataArray).slice(0, 64);
    
    // 继续下一帧
    animationFrameId = requestAnimationFrame(updateVisualization);
  }
  
  // 重新连接音频
  async function reconnectAudio() {
    try {
      if (audioElement) {
        audioElement.pause();
      }
      await initAudio();
    } catch (error) {
      console.error('重新连接音频失败:', error);
      // 5秒后重试
      setTimeout(reconnectAudio, 5000);
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
    
    // 监听Rust后端发送的歌曲信息更新事件
    const unlisten = listen<SongInfo>('song-info-update', (event) => {
      currentSong = {
        title: event.payload.title || 'Unknown Title',
        artist: event.payload.artist || 'Unknown Artist'
      };
    });
    
    // 定期检查音频状态
    const audioCheckInterval = setInterval(() => {
      if (audioContext && audioContext.state === 'suspended') {
        audioContext.resume();
      }
      
      // 如果音频停止播放，尝试重新连接
      if (audioElement && audioElement.readyState === 0 && isPlaying) {
        console.log('检测到音频连接断开，尝试重新连接...');
        reconnectAudio();
      }
    }, 10000); // 每10秒检查一次
    
    // 返回清理函数
    return () => {
      unlisten.then(f => f()); // 清理事件监听器
      clearInterval(audioCheckInterval);
      
      if (animationFrameId) {
        cancelAnimationFrame(animationFrameId);
      }
      
      if (audioElement) {
        audioElement.pause();
        audioElement = null;
      }
      
      if (audioContext) {
        audioContext.close();
        audioContext = null;
      }
    };
  });
</script>

<div class="container" onmousedown={startDrag} role="button" tabindex="0">
  <div class="header">
    <div class="controls">
      <button class="control-btn" onclick={() => setAlwaysOnTop(!isPinned)}>
        {isPinned ? '🔓' : '🔒'}
      </button>
      <button class="control-btn" onclick={() => setMousePassthrough(!isMousePassthrough)}>
        {isMousePassthrough ? '🖱️' : '✋'}
      </button>
      <button class="control-btn" onclick={minimizeWindow}>−</button>
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
</div>

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
<script lang="ts">
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { AudioAnalysis } from '$lib/visualizations/wavtools';
  import BarVisualizer from '$lib/visualizations/core/BarVisualizer.svelte';

  // 定义歌曲信息类型
  interface SongInfo {
    title: string;
    artist: string;
  }

  // 状态变量
  let audioElement: HTMLAudioElement | null = null;
  let audioAnalysis: AudioAnalysis | null = null;
  let animationFrameId: number | null = null;
  let frequencyValues = $state(new Float32Array(64));
  let currentSong = $state({ title: 'Loading...', artist: '' });
  let themeColor = $state('#3498db');
  let isPinned = $state(true);
  let isMousePassthrough = $state(false);
  let streamUrl = 'https://radio.startend.xyz/radio';
  let isPlaying = $state(false);
  
  // 获取当前窗口实例
  const appWindow = getCurrentWindow();
  
  // 初始化音频播放器
  async function initAudio() {
    try {
      // 清理现有资源
      if (animationFrameId) {
        cancelAnimationFrame(animationFrameId);
      }
      
      if (audioElement) {
        audioElement.pause();
        audioElement = null;
      }
      
      // 创建新的音频元素
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
      
      // 创建音频分析器
      audioAnalysis = new AudioAnalysis(audioElement);
      
      // 尝试恢复音频上下文
      await audioAnalysis.resumeIfSuspended();
      
      // 开始播放
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
    if (!audioAnalysis) return;
    
    try {
      // 恢复音频上下文（如果被挂起）
      audioAnalysis.resumeIfSuspended();
      
      // 获取频率数据
      const frequencyData = audioAnalysis.getFrequencies('frequency', -100, -30);
      
      // 更新条形图数据（取前64个频率桶）
      const values = frequencyData.values;
      // 确保我们有正确的长度
      if (values.length >= 64) {
        frequencyValues = values.slice(0, 64);
      } else {
        // 如果数据不足，用0填充
        const paddedValues = new Float32Array(64);
        paddedValues.set(values);
        frequencyValues = paddedValues;
      }
      
      // 继续下一帧
      animationFrameId = requestAnimationFrame(updateVisualization);
    } catch (error) {
      console.error('可视化更新失败:', error);
      // 即使出错也继续更新
      animationFrameId = requestAnimationFrame(updateVisualization);
    }
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
      
      if (audioAnalysis) {
        audioAnalysis = null;
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
  
  <!-- 使用核心音频可视化组件 -->
  <div class="visualization-container">
    <BarVisualizer values={frequencyValues} color={themeColor} />
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
  
  .visualization-container {
    height: 60px;
    margin-bottom: 10px;
    width: 100%;
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
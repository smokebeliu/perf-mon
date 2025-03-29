import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

type ProcessInfo = {
  pid: number;
  name: string;
  cpu_usage: number;
  memory: number;
}

type PerfInfo = {
  time: Date;
  system: {
    name: string;
    hostname: string;
  };
  cpu: number[];
  memory: {
    total: number;
    used: number;
    total_swap: number;
    used_swap: number;
  };
  processes: ProcessInfo[];
}

type BufferStatus = {
  last_item: PerfInfo | null;
  buffer_size: number;
  total_sent: number;
}

const ProcessTable = () => {
  const [status, setStatus] = useState<BufferStatus | null>(null);

  const fetchStatus = async () => {
    try {
      const data = await invoke('get_status') as BufferStatus;
      setStatus(data);
      console.log('status', data);
    } catch (error) {
      console.error('Ошибка получения статуса:', error);
    }
  };

  useEffect(() => {
    fetchStatus();
    const intervalId = setInterval(fetchStatus, 10000);
    return () => clearInterval(intervalId);
  }, []);

  if (!status?.last_item) {
    return <div>Загрузка данных...</div>;
  }

  const perfInfo = status.last_item;
  const processes = perfInfo.processes
    .filter((item) => item.cpu_usage !== 0)
    .map(item => ({
      ...item,
      memory: item.memory / 1024 / 1024
    }))
    .sort((a, b) => b.cpu_usage - a.cpu_usage);

  return (
    <div>
      <h1>Системная информация</h1>
      <div style={{ marginBottom: '20px' }}>
        <h2>Статус буфера</h2>
        <p>Размер буфера: {status.buffer_size}</p>
        <p>Всего отправлено: {status.total_sent}</p>
      </div>

      <div style={{ marginBottom: '20px' }}>
        <h2>Система</h2>
        <p>Имя системы: {perfInfo.system.name}</p>
        <p>Имя хоста: {perfInfo.system.hostname}</p>
      </div>

      <div style={{ marginBottom: '20px' }}>
        <h2>CPU</h2>
        <p>Загрузка ядер CPU:</p>
        <div style={{ display: 'flex', gap: '10px', flexWrap: 'wrap' }}>
          {perfInfo.cpu.map((usage, index) => (
            <div key={index}>
              Ядро {index}: {usage.toFixed(2)}%
            </div>
          ))}
        </div>
      </div>

      <div style={{ marginBottom: '20px' }}>
        <h2>Память</h2>
        <p>Всего памяти: {(perfInfo.memory.total / 1024 / 1024 / 1024).toFixed(2)} ГБ</p>
        <p>Использовано памяти: {(perfInfo.memory.used / 1024 / 1024 / 1024).toFixed(2)} ГБ</p>
        <p>Всего swap: {(perfInfo.memory.total_swap / 1024 / 1024 / 1024).toFixed(2)} ГБ</p>
        <p>Использовано swap: {(perfInfo.memory.used_swap / 1024 / 1024 / 1024).toFixed(2)} ГБ</p>
      </div>

      <h1>Текущая информация о процессах</h1>
      <table style={{ borderCollapse: 'collapse', width: '100%' }}>
        <thead>
          <tr>
            <th style={{ border: '1px solid #333', padding: '5px' }}>PID</th>
            <th style={{ border: '1px solid #333', padding: '5px' }}>Имя процесса</th>
            <th style={{ border: '1px solid #333', padding: '5px' }}>CPU Usage (%)</th>
            <th style={{ border: '1px solid #333', padding: '5px' }}>Memory (Mb)</th>
          </tr>
        </thead>
        <tbody>
          {processes.map((proc: ProcessInfo) => (
            <tr key={proc.pid}>
              <td style={{ border: '1px solid #333', padding: '5px' }}>{proc.pid}</td>
              <td style={{ border: '1px solid #333', padding: '5px' }}>{proc.name}</td>
              <td style={{ border: '1px solid #333', padding: '5px' }}>{proc.cpu_usage.toFixed(2)}</td>
              <td style={{ border: '1px solid #333', padding: '5px' }}>{proc.memory.toFixed(2)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

export default ProcessTable;
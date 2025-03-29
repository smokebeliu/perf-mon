import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

type ProcessInfo = {
  pid: number;
  name: string;
  cpu_usage: number;
  memory: number;
}

const ProcessTable = () => {
  const [processes, setProcesses] = useState<ProcessInfo[]>([]);

  const fetchProcesses = async () => {
    try {
      const data = await invoke('get_current_process_info') as ProcessInfo[];
      console.log('data', data.filter((item) => item.cpu_usage !== 0));
      setProcesses(data.map(item => ({
        ...item,
         memory: item.memory / 1024 / 1024
      })).sort((a, b) => b.cpu_usage - a.cpu_usage));
    } catch (error) {
      console.error('Ошибка получения данных:', error);
    }
  };

  useEffect(() => {
    // Получаем данные сразу при монтировании компонента
    fetchProcesses();
    // Обновляем данные каждые 5 секунд
    const intervalId = setInterval(fetchProcesses, 65000);
    return () => clearInterval(intervalId);
  }, []);

  return (
    <div>
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
using System.IO.Ports;
using FanControl.Plugins;

namespace FanControl.DexControllerSensor
{
    public class DexPlugin : IPlugin2
    {
        public string Name => "DexFan Controller";

        private SerialPort local_port;

        public void Close()
        {
            local_port.Close();
        }

        public void Initialize()
        {
            local_port = new SerialPort
            {
                PortName = "COM6"
            };

        }

        public void Load(IPluginSensorsContainer _container)
        {
            throw new NotImplementedException();
        }

        public void Update()
        {
            throw new NotImplementedException();
        }
    }
}

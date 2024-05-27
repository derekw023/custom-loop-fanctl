using System.IO.Ports;
using FanControl.Plugins;

namespace FanControl.DexControllerSensor
{
    public class DexPlugin : IPlugin2
    {
        public string Name => "DexFan Water Sensor";

        static SerialPort? port;

        WaterSensor? sensor_temp;
        private IPluginLogger logger;
        private IPluginDialog dialog;


        public void Close()
        {
            port?.Close();
        }

        public DexPlugin(IPluginLogger a_logger, IPluginDialog a_dialog)
        {
            logger = a_logger;
            dialog = a_dialog;
        }

        public void Initialize()
        {

            try { 
                port = new SerialPort("COM14");
                port.Open();
            } catch (IOException ex) {
                logger.Log($"Failed to open serial port with exception {ex}");
                port = null;
            }

        }

        public void Load(IPluginSensorsContainer container)
        {
            sensor_temp = new() { };
            container.TempSensors.Add(sensor_temp);


        }

        public void Update()
        {
            try
            {
                port?.WriteLine("t");
                String? msg = port?.ReadLine();
                if (msg != null) { logger.Log(msg); }
            } catch (SystemException ex) 
            {
                logger.Log("Exception while reading serial port: " + ex.Message);
                if (ex.Message == "The port is closed.") { port = null; }
            }
            
            if (sensor_temp != null)
            {
                sensor_temp.Value = 42;
            }
        }
    }

    public class WaterSensor : IPluginSensor
    {
        public string Id => "DexControllerWaterTempCS";

        public string Name => "Water Temperature (C)";

        public float? Value
        {
            get; set;
        }

        // Updates from plugin context
        public void Update()
        {
        }

    }
}
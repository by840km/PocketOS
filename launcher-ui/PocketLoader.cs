using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Markup;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Imaging;
using System.Windows.Threading;

public class PocketLoader : Application {
 Window window; TextBlock status, terminal, clock, hint; Button retry; FrameworkElement progress; Border[] steps; Expander details;
 string diagnostic; Process core; IntPtr job; bool ready, removed, closing; string errors=""; string[] arguments;
 readonly List<string> lines=new List<string>(); readonly Stopwatch elapsed=new Stopwatch(); DispatcherTimer timer;
 [STAThread] public static void Main(string[] args) { var app=new PocketLoader();app.arguments=args;app.Run(); }
 protected override void OnStartup(StartupEventArgs e) {
  base.OnStartup(e);
  var filtered=new List<string>();for(int i=0;i<arguments.Length;i++){if(arguments[i]=="--diagnostics" && i+1<arguments.Length){diagnostic=arguments[++i];}else filtered.Add(arguments[i]);}arguments=filtered.ToArray();
  window=(Window)XamlReader.Parse(@"
<Window xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' Title='PocketOS' WindowStyle='None' WindowState='Maximized' ResizeMode='NoResize' Background='#1E1E2E' Topmost='True' FontFamily='Segoe UI' Foreground='#CDD6F4' UseLayoutRounding='True'>
 <Window.Resources>
  <Style TargetType='Button'><Setter Property='Foreground' Value='#CDD6F4'/><Setter Property='Background' Value='#181825'/><Setter Property='BorderBrush' Value='#585B70'/><Setter Property='BorderThickness' Value='1'/><Setter Property='Padding' Value='18,10'/><Setter Property='FontSize' Value='14'/><Setter Property='MinHeight' Value='42'/><Setter Property='Template'><Setter.Value><ControlTemplate TargetType='Button'><Border Background='{TemplateBinding Background}' BorderBrush='{TemplateBinding BorderBrush}' BorderThickness='{TemplateBinding BorderThickness}' CornerRadius='5' Padding='{TemplateBinding Padding}'><ContentPresenter HorizontalAlignment='Center' VerticalAlignment='Center' RecognizesAccessKey='True'/></Border></ControlTemplate></Setter.Value></Setter><Style.Triggers><Trigger Property='IsMouseOver' Value='True'><Setter Property='Background' Value='#313244'/></Trigger><Trigger Property='IsPressed' Value='True'><Setter Property='Background' Value='#45475A'/></Trigger><Trigger Property='IsKeyboardFocused' Value='True'><Setter Property='BorderBrush' Value='#89B4FA'/><Setter Property='BorderThickness' Value='2'/></Trigger></Style.Triggers></Style>
 </Window.Resources>
 <Grid Margin='40,32'>
  <Grid.RowDefinitions><RowDefinition Height='Auto'/><RowDefinition Height='*'/><RowDefinition Height='Auto'/></Grid.RowDefinitions>
  <DockPanel LastChildFill='False'>
   <StackPanel Orientation='Horizontal' DockPanel.Dock='Left'>
    <Viewbox Width='27' Height='29' Margin='0,0,12,0'><Canvas Width='28' Height='30'><Path Stroke='#89B4FA' StrokeThickness='2.5' StrokeLineJoin='Round' Data='M 3,2 L 25,2 L 25,19 C 25,26 14,29 14,29 C 14,29 3,26 3,19 Z'/><Path Stroke='#89B4FA' StrokeThickness='2' Data='M 9,11 L 19,11 M 9,16 L 16,16'/></Canvas></Viewbox>
    <TextBlock Text='PocketOS' FontFamily='Bahnschrift' FontSize='26' VerticalAlignment='Center'/>
   </StackPanel>
   <TextBlock Name='Clock' Text='00:00' Foreground='#A6ADC8' FontSize='13' DockPanel.Dock='Right' VerticalAlignment='Center' AutomationProperties.Name='Время запуска'/>
  </DockPanel>
  <Grid Grid.Row='1' MaxWidth='620' Margin='0,36' VerticalAlignment='Center'>
   <StackPanel>
    <TextBlock Name='Status' Text='Запускаем KDE Plasma' FontSize='26' FontWeight='SemiBold' TextWrapping='Wrap' Margin='0,0,0,12' AutomationProperties.LiveSetting='Polite'/>
    <TextBlock Text='Рабочий стол откроется автоматически.' FontSize='15' Foreground='#A6ADC8' Margin='0,0,0,36' TextWrapping='Wrap'/>
    <UniformGrid Name='Progress' Columns='4' Margin='0,0,-12,28'>
     <StackPanel Margin='0,0,12,0'><Border Name='Step0' Height='3' Background='#45475A' CornerRadius='1'/><TextBlock Text='Окружение' Foreground='#A6ADC8' Margin='0,10,0,0' FontSize='12'/></StackPanel>
     <StackPanel Margin='0,0,12,0'><Border Name='Step1' Height='3' Background='#45475A' CornerRadius='1'/><TextBlock Text='Ядро' Foreground='#A6ADC8' Margin='0,10,0,0' FontSize='12'/></StackPanel>
     <StackPanel Margin='0,0,12,0'><Border Name='Step2' Height='3' Background='#45475A' CornerRadius='1'/><TextBlock Text='Системный диск' Foreground='#A6ADC8' Margin='0,10,0,0' FontSize='12'/></StackPanel>
     <StackPanel Margin='0,0,12,0'><Border Name='Step3' Height='3' Background='#45475A' CornerRadius='1'/><TextBlock Text='Рабочий стол' Foreground='#A6ADC8' Margin='0,10,0,0' FontSize='12'/></StackPanel>
    </UniformGrid>
    <Expander Name='Details' Header='Подробности' Foreground='#A6ADC8' FontSize='13'>
     <ScrollViewer MaxHeight='220' VerticalScrollBarVisibility='Auto' Margin='0,14,0,0'><TextBlock Name='Terminal' Text='Проверяем файлы системы…' FontSize='13' LineHeight='23' TextWrapping='Wrap'/></ScrollViewer>
    </Expander>
    <TextBlock Name='Hint' Text='' Foreground='#A6ADC8' FontSize='14' Margin='0,18,0,0' TextWrapping='Wrap'/>
    <Button Name='Retry' Content='Повторить с совместимой графикой' Visibility='Collapsed' HorizontalAlignment='Left' Margin='0,18,0,0'/>
   </StackPanel>
  </Grid>
  <DockPanel Grid.Row='2' LastChildFill='False'>
   <TextBlock Text='Временный сеанс. Файлы пока не сохраняются после выхода.' Foreground='#A6ADC8' FontSize='12' MaxWidth='580' TextWrapping='Wrap' DockPanel.Dock='Left' VerticalAlignment='Center' Margin='0,0,24,0'/>
   <Button Name='Cancel' Content='_Отмена' DockPanel.Dock='Right' IsCancel='True'/>
  </DockPanel>
 </Grid>
</Window>");
  status=(TextBlock)window.FindName("Status");terminal=(TextBlock)window.FindName("Terminal");clock=(TextBlock)window.FindName("Clock");hint=(TextBlock)window.FindName("Hint");retry=(Button)window.FindName("Retry");progress=(FrameworkElement)window.FindName("Progress");
  steps=new Border[4];for(int i=0;i<4;i++)steps[i]=(Border)window.FindName("Step"+i);details=(Expander)window.FindName("Details");
  ((Button)window.FindName("Cancel")).Click+=(s,a)=>window.Close();
  retry.Click+=(s,a)=>StartCore(true);
  window.Closing+=(s,a)=>{closing=true;StopCore();};
  timer=new DispatcherTimer();timer.Interval=TimeSpan.FromMilliseconds(250);timer.Tick+=(s,a)=>clock.Text=elapsed.Elapsed.ToString(@"mm\:ss");timer.Start();
  window.Loaded+=(s,a)=>{
   if(arguments.Length>0 && arguments[0]=="--preview") {
    status.Text="Подготавливаем рабочий стол";SetStep(3);terminal.Text="Комплект системы проверен\nЯдро запущено\nЗапускаем KDE Plasma…";
    var capture=new DispatcherTimer();capture.Interval=TimeSpan.FromSeconds(1);capture.Tick+=(x,y)=>{capture.Stop();if(arguments.Length>1)SavePreview(arguments[1]);window.Close();};capture.Start();
   } else StartCore(false);
  };
  window.Show();
 }
 void SavePreview(string path) {
  window.UpdateLayout();var visual=(FrameworkElement)window;var image=new RenderTargetBitmap((int)visual.ActualWidth,(int)visual.ActualHeight,96,96,PixelFormats.Pbgra32);image.Render(visual);var encoder=new PngBitmapEncoder();encoder.Frames.Add(BitmapFrame.Create(image));using(var stream=File.Create(path))encoder.Save(stream);
 }
 void SetStep(int active) {for(int i=0;i<4;i++)steps[i].Background=new SolidColorBrush((Color)ColorConverter.ConvertFromString(i<active?"#89B4FA":i==active?"#CDD6F4":"#45475A"));}
 void Append(string text) {lines.Add(text);while(lines.Count>5)lines.RemoveAt(0);terminal.Text=String.Join("\n",lines.ToArray());}
 void Stage(string line) {
  if(line.Contains("USB removed"))removed=true;
  if(!line.StartsWith("UI:"))return;
  var stage=line.Substring(3);Report(stage);
  switch(stage){case "START":SetStep(0);break;case "KERNEL":SetStep(1);break;case "ROOT":SetStep(2);break;case "DESKTOP_START":SetStep(3);break;case "READY":SetStep(4);break;}
  switch(stage) {
   case "START":status.Text="Запускаем систему";Append("✓ Комплект системы проверен");Append("› Запускаем виртуальное окружение…");break;
   case "KERNEL":status.Text="Ядро запущено";Append("✓ Ядро Linux запущено");break;
   case "ROOT":status.Text="Подключаем рабочее пространство";Append("✓ Системный диск подключён");break;
   case "DESKTOP_START":status.Text="Подготавливаем рабочий стол";Append("› Запускаем KDE Plasma…");break;
   case "RETRY":status.Text="Пробуем совместимый графический режим";Append("› Перезапуск графики в режиме совместимости");break;
   case "READY":ready=true;status.Text="Всё готово";Append("✓ Рабочий стол готов");timer.Stop();if(diagnostic!=null)SavePreview(diagnostic+".png");if(!SystemParameters.ClientAreaAnimation){window.Hide();break;}var fade=new DoubleAnimation(1,0,TimeSpan.FromMilliseconds(180));fade.Completed+=(s,a)=>{if(!closing)window.Hide();};window.BeginAnimation(UIElement.OpacityProperty,fade);break;
   default:if(stage.StartsWith("ERROR")){Append("! "+stage.Substring(5).TrimStart(':'));}break;
  }
 }
 void StartCore(bool software) {
  StopCore();ready=false;removed=false;errors="";lines.Clear();window.BeginAnimation(UIElement.OpacityProperty,null);window.Opacity=1;window.Topmost=true;window.Show();retry.Visibility=Visibility.Collapsed;progress.Visibility=Visibility.Visible;status.Text="Запускаем KDE Plasma";hint.Text="";details.IsExpanded=false;SetStep(-1);Append("› Проверяем комплект системы…");elapsed.Restart();timer.Start();
  try {
   var args=new List<string>(arguments);if(software){args.Clear();args.Add("--graphics");args.Add("software");}
   var start=new ProcessStartInfo(Path.Combine(AppDomain.CurrentDomain.BaseDirectory,"pocketos-core.exe"));start.WorkingDirectory=AppDomain.CurrentDomain.BaseDirectory;start.UseShellExecute=false;start.CreateNoWindow=true;start.RedirectStandardOutput=true;start.RedirectStandardError=true;start.StandardOutputEncoding=Encoding.UTF8;start.StandardErrorEncoding=Encoding.UTF8;
   start.Arguments=String.Join(" ",args.ConvertAll(Quote).ToArray());
   var local=new Process();local.StartInfo=start;local.EnableRaisingEvents=true;core=local;
   local.OutputDataReceived+=(s,e)=>{if(e.Data!=null)Post(()=>{if(core==local)Stage(e.Data);});};
   local.ErrorDataReceived+=(s,e)=>{if(e.Data!=null)Post(()=>{if(core==local){errors+=e.Data+"\n";if(errors.Length>6000)errors=errors.Substring(errors.Length-6000);}});};
   local.Exited+=(s,e)=>{try{local.WaitForExit();}catch{}Post(()=>{if(core!=local||closing)return; if(ready||removed){window.Close();}else Fail(errors.Length>0?errors:"Запуск завершился до готовности рабочего стола.");});};
   if(!local.Start())throw new Exception("Не удалось запустить ядро загрузчика.");job=CreateJobObject(IntPtr.Zero,null);if(job==IntPtr.Zero)throw new Win32Exception();
   var limits=new Limits();limits.Basic.Flags=0x2000;int size=Marshal.SizeOf(typeof(Limits));IntPtr ptr=Marshal.AllocHGlobal(size);try{Marshal.StructureToPtr(limits,ptr,false);if(!SetInformationJobObject(job,9,ptr,(uint)size)||!AssignProcessToJobObject(job,local.Handle))throw new Win32Exception();}finally{Marshal.FreeHGlobal(ptr);}
   local.BeginOutputReadLine();local.BeginErrorReadLine();
  } catch(Exception error){StopCore();Fail(error.Message);}
 }
 void Report(string text) {if(diagnostic!=null){try{File.AppendAllText(diagnostic,DateTime.UtcNow.ToString("o")+" "+text+Environment.NewLine);}catch{}}}
 void Fail(string text) {Report("ERROR "+text);timer.Stop();window.BeginAnimation(UIElement.OpacityProperty,null);window.Opacity=1;window.Show();status.Text="Не удалось запустить систему";progress.Visibility=Visibility.Collapsed;details.IsExpanded=true;terminal.Text="! "+(text.Length>1400?text.Substring(0,600)+"\n…\n"+text.Substring(text.Length-700):text);hint.Text="Можно повторить запуск с программной графикой.";retry.Visibility=Visibility.Visible;}
 void StopCore() {var previous=core;core=null;if(job!=IntPtr.Zero){CloseHandle(job);job=IntPtr.Zero;}if(previous!=null){try{if(!previous.HasExited)previous.Kill();}catch{}}}
 void Post(Action action) {if(!Dispatcher.HasShutdownStarted)Dispatcher.BeginInvoke(action);}
 static string Quote(string value){var b=new StringBuilder("\"");int slashes=0;foreach(char c in value){if(c=='\\'){slashes++;continue;}if(c=='\"'){b.Append('\\',slashes*2+1);b.Append(c);}else{b.Append('\\',slashes);b.Append(c);}slashes=0;}b.Append('\\',slashes*2);b.Append('"');return b.ToString();}
 [StructLayout(LayoutKind.Sequential)] struct BasicLimits{public long ProcessTime,JobTime;public uint Flags;public UIntPtr MinWs,MaxWs;public uint Active;public UIntPtr Affinity;public uint Priority,Scheduling;}
 [StructLayout(LayoutKind.Sequential)] struct IoCounters{public ulong A,B,C,D,E,F;}
 [StructLayout(LayoutKind.Sequential)] struct Limits{public BasicLimits Basic;public IoCounters Io;public UIntPtr ProcessMem,JobMem,PeakProcess,PeakJob;}
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode,EntryPoint="CreateJobObjectW")]static extern IntPtr CreateJobObject(IntPtr attr,string name);
 [DllImport("kernel32.dll",SetLastError=true)]static extern bool SetInformationJobObject(IntPtr job,int type,IntPtr data,uint size);
 [DllImport("kernel32.dll",SetLastError=true)]static extern bool AssignProcessToJobObject(IntPtr job,IntPtr process);
 [DllImport("kernel32.dll")]static extern bool CloseHandle(IntPtr handle);
}

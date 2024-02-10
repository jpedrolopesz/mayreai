import cv2
import dlib

# Iniciar a captura de vídeo
cap = cv2.VideoCapture(1)  # Ajuste o índice da sua webcam, se necessário

# Carregar o detector de faces da Dlib
detector = dlib.get_frontal_face_detector()
# Carregar o preditor de pontos de referência facial pré-treinado
predictor = dlib.shape_predictor('data/shape_predictor_68_face_landmarks.dat')

while True:
    # Capturar frame-a-frame
    ret, frame = cap.read()
    if not ret:
        print("Falha ao capturar imagem. Saindo...")
        break


    frame = cv2.flip(frame, 1)

    # Converter o frame para escala de cinza para a detecção
    gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)

    # Detectar faces no frame
    faces = detector(gray)
    for face in faces:
        # Encontrar os pontos de referência faciais para cada face detectada
        landmarks = predictor(gray, face)

        # Desenhar os pontos de referência facial no frame
        for n in range(0, 68):
            x = landmarks.part(n).x
            y = landmarks.part(n).y
            cv2.circle(frame, (x, y), 3, (255, 0, 0), -1)
            cv2.putText(frame, str(n), (x, y), cv2.FONT_HERSHEY_SIMPLEX, 0.3, (255, 255, 255), 1)  # Adicionar número


    # Exibir o frame com os pontos de referência desenhados
    cv2.imshow('Webcam', frame)

    # Fechar a janela com a tecla 'q'
    if cv2.waitKey(1) & 0xFF == ord('q'):
        break

# Quando tudo estiver feito, liberar a captura
cap.release()
cv2.destroyAllWindows()
